use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time;

/// Global price cache with automatic background refresh
static PRICE_MANAGER: Lazy<PriceManager> = Lazy::new(|| PriceManager::new());

/// Cached price entry with timestamp
#[derive(Clone, Debug)]
struct PriceEntry {
    price: f64,
    change_24h: f64,
    last_updated: Instant,
}

/// Thread-safe price cache manager
pub struct PriceManager {
    cache: Arc<Mutex<HashMap<String, PriceEntry>>>,
    refresh_interval: Duration,
}

impl PriceManager {
    fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            refresh_interval: Duration::from_secs(60),
        }
    }

    /// Get cached price for a symbol (read-only, never triggers fetch)
    pub fn get_cached_price(&self, symbol: &str) -> Option<f64> {
        // Handle stablecoins immediately
        if let Some(price) = get_stablecoin_price(symbol) {
            return Some(price);
        }

        let cache = self.cache.lock().unwrap();
        cache.get(symbol).map(|entry| entry.price)
    }

    /// Get cached 24h change for a symbol
    pub fn get_cached_24h_change(&self, symbol: &str) -> Option<f64> {
        // Stablecoins have 0 change
        if get_stablecoin_price(symbol).is_some() {
            return Some(0.0);
        }

        let cache = self.cache.lock().unwrap();
        cache.get(symbol).map(|entry| entry.change_24h)
    }

    /// Force refresh all prices (for manual refresh button)
    pub async fn force_refresh(&self) -> Result<(), String> {
        self.refresh_prices().await
    }

    /// Internal: Refresh all prices from API
    async fn refresh_prices(&self) -> Result<(), String> {
        // List of all symbols we track
        let symbols = vec![
            "BTC".to_string(),
            "ETH".to_string(),
            "USDT".to_string(),
            "USDC".to_string(),
            "DAI".to_string(),
            "MATIC".to_string(),
            "BNB".to_string(),
        ];

        // Fetch fresh prices
        let fresh_prices = super::price::fetch_prices(symbols).await?;

        // Update cache
        let mut cache = self.cache.lock().unwrap();
        let now = Instant::now();
        
        for (symbol, data) in fresh_prices {
            cache.insert(symbol, PriceEntry {
                price: data.0,
                change_24h: data.1,
                last_updated: now,
            });
        }

        tracing::info!(
            symbols_updated = cache.len(),
            "Price cache refreshed successfully"
        );

        Ok(())
    }
}

/// Check if a symbol is a stablecoin and return its fixed price
fn get_stablecoin_price(symbol: &str) -> Option<f64> {
    match symbol.to_uppercase().as_str() {
        "USDT" | "USDC" | "DAI" => Some(1.0),
        _ => None,
    }
}

/// Start background price refresh task
pub async fn start_background_refresh() {
    // Initial refresh
    if let Err(e) = PRICE_MANAGER.refresh_prices().await {
        tracing::warn!(error = %e, "Initial price refresh failed");
    } else {
        tracing::info!("Price manager initialized with initial prices");
    }

    // Periodic refresh
    let mut interval = time::interval(PRICE_MANAGER.refresh_interval);
    loop {
        interval.tick().await;
        
        if let Err(e) = PRICE_MANAGER.refresh_prices().await {
            tracing::warn!(error = %e, "Background price refresh failed");
        }
    }
}

/// Get cached price for a symbol (public API for business logic)
pub fn get_cached_price(symbol: &str) -> Option<f64> {
    PRICE_MANAGER.get_cached_price(symbol)
}

/// Get cached 24h change for a symbol (public API)
pub fn get_cached_24h_change(symbol: &str) -> Option<f64> {
    PRICE_MANAGER.get_cached_24h_change(symbol)
}

/// Force refresh all prices (public API for manual refresh)
pub async fn force_refresh_prices() -> Result<(), String> {
    PRICE_MANAGER.force_refresh().await
}

#[tauri::command]
pub async fn refresh_all_prices() -> Result<String, String> {
    force_refresh_prices().await?;
    Ok("Prices refreshed successfully".to_string())
}
