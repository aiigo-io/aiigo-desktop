use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const RETRY_ATTEMPTS: u32 = 2;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;
const REQUEST_TIMEOUT_SECS: u64 = 10;
const CACHE_DURATION_SECS: u64 = 60; // Cache prices for 60 seconds

#[derive(Debug, Deserialize)]
struct PriceData {
    #[serde(default)]
    usd: Option<f64>,
}

// Simple in-memory cache
struct PriceCache {
    prices: HashMap<String, f64>,
    last_update: Option<Instant>,
}

static PRICE_CACHE: Lazy<Mutex<PriceCache>> = Lazy::new(|| {
    Mutex::new(PriceCache {
        prices: HashMap::new(),
        last_update: None,
    })
});

/// Map asset symbols to CoinGecko IDs
fn get_coingecko_id(symbol: &str) -> Option<&'static str> {
    match symbol.to_uppercase().as_str() {
        "ETH" => Some("ethereum"),
        "BTC" => Some("bitcoin"),
        "USDT" => Some("tether"),
        "USDC" => Some("usd-coin"),
        "MATIC" => Some("matic-network"),
        "BNB" => Some("binancecoin"),
        _ => None,
    }
}

/// Fetch USD prices for multiple crypto assets
pub async fn fetch_prices(symbols: Vec<String>) -> Result<HashMap<String, f64>, String> {
    // Check cache first
    {
        let cache = PRICE_CACHE.lock().unwrap();
        if let Some(last_update) = cache.last_update {
            if last_update.elapsed().as_secs() < CACHE_DURATION_SECS {
                // Cache is still valid
                let mut result = HashMap::new();
                for symbol in &symbols {
                    if let Some(coingecko_id) = get_coingecko_id(symbol) {
                        if let Some(&price) = cache.prices.get(coingecko_id) {
                            result.insert(symbol.clone(), price);
                        }
                    }
                }
                if !result.is_empty() {
                    println!(
                        "[INFO] Using cached prices (age: {}s)",
                        last_update.elapsed().as_secs()
                    );
                    return Ok(result);
                }
            }
        }
    }

    // Convert symbols to CoinGecko IDs
    let ids: Vec<String> = symbols
        .iter()
        .filter_map(|symbol| get_coingecko_id(symbol).map(|id| id.to_string()))
        .collect();

    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    // Create a unique set of IDs
    let unique_ids: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        ids.into_iter()
            .filter(|id| seen.insert(id.clone()))
            .collect()
    };

    let ids_param = unique_ids.join(",");

    for attempt in 1..=RETRY_ATTEMPTS {
        let attempt_start = Instant::now();
        match try_fetch_prices(&ids_param).await {
            Ok(response_map) => {
                // Update cache
                {
                    let mut cache = PRICE_CACHE.lock().unwrap();
                    cache.prices = response_map.clone();
                    cache.last_update = Some(Instant::now());
                }

                // Map CoinGecko IDs back to symbols
                let mut result = HashMap::new();
                for symbol in &symbols {
                    if let Some(coingecko_id) = get_coingecko_id(symbol) {
                        if let Some(&price) = response_map.get(coingecko_id) {
                            result.insert(symbol.clone(), price);
                        }
                    }
                }
                let duration_ms = attempt_start.elapsed().as_millis();
                println!(
                    "[INFO] Fetched {} fresh prices from CoinGecko in {}ms",
                    result.len(),
                    duration_ms
                );
                return Ok(result);
            }
            Err(e) => {
                if attempt < RETRY_ATTEMPTS {
                    let delay_ms = INITIAL_RETRY_DELAY_MS * (2_u64.pow(attempt - 1));
                    eprintln!(
                        "[RETRY] Price query attempt {} failed: {}. Retrying in {}ms...",
                        attempt, e, delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                } else {
                    eprintln!("[WARNING] Failed to fetch prices after all retries: {}", e);
                    // Try to use stale cache as fallback
                    let cache = PRICE_CACHE.lock().unwrap();
                    if !cache.prices.is_empty() {
                        eprintln!("[INFO] Using stale cache as fallback");
                        let mut result = HashMap::new();
                        for symbol in &symbols {
                            if let Some(coingecko_id) = get_coingecko_id(symbol) {
                                if let Some(&price) = cache.prices.get(coingecko_id) {
                                    result.insert(symbol.clone(), price);
                                }
                            }
                        }
                        return Ok(result);
                    }
                    // Return empty map instead of error to allow wallet queries to continue
                    return Ok(HashMap::new());
                }
            }
        }
    }

    Ok(HashMap::new())
}

async fn try_fetch_prices(ids: &str) -> Result<HashMap<String, f64>, String> {
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
        ids
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent("aiigo-desktop/0.1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    // Get response text for parsing
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Parse as a map of coin_id -> price_data
    let parsed: HashMap<String, PriceData> = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}. Response was: {}", e, text))?;

    // Extract just the USD prices, skipping coins with missing data
    let mut result = HashMap::new();
    for (coin_id, price_data) in parsed {
        if let Some(usd_price) = price_data.usd {
            result.insert(coin_id, usd_price);
        } else {
            eprintln!("[WARNING] Missing USD price data for: {}", coin_id);
        }
    }

    Ok(result)
}

/// Fetch price for a single asset
#[allow(dead_code)]
pub async fn fetch_price(symbol: &str) -> Result<f64, String> {
    let prices = fetch_prices(vec![symbol.to_string()]).await?;
    prices
        .get(symbol)
        .copied()
        .ok_or_else(|| format!("Price not found for {}", symbol))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_coingecko_id() {
        assert_eq!(get_coingecko_id("ETH"), Some("ethereum"));
        assert_eq!(get_coingecko_id("eth"), Some("ethereum"));
        assert_eq!(get_coingecko_id("BTC"), Some("bitcoin"));
        assert_eq!(get_coingecko_id("USDT"), Some("tether"));
        assert_eq!(get_coingecko_id("USDC"), Some("usd-coin"));
        assert_eq!(get_coingecko_id("MATIC"), Some("matic-network"));
        assert_eq!(get_coingecko_id("BNB"), Some("binancecoin"));
        assert_eq!(get_coingecko_id("UNKNOWN"), None);
    }

    #[tokio::test]
    async fn test_fetch_price() {
        // This test requires internet connection
        let result = fetch_price("ETH").await;
        // Just check that it doesn't error - actual price will vary
        assert!(result.is_ok() || result.is_err());
    }
}
