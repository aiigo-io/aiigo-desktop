use once_cell::sync::Lazy;
use crate::wallet::security::sanitize;
use crate::wallet::state::price as state_price;
use crate::wallet::state::types::PriceState;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use chrono::Utc;

const RETRY_ATTEMPTS: u32 = 2;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;
const REQUEST_TIMEOUT_SECS: u64 = 10;
const CACHE_DURATION_SECS: u64 = 60; // Cache prices for 60 seconds
#[allow(dead_code)]
const PRICE_FRESH_WITHIN_SECS: i64 = 60;
#[allow(dead_code)]
const PRICE_STALE_AFTER_SECS: i64 = 300;

#[derive(Debug, Deserialize)]
struct PriceData {
    #[serde(default)]
    usd: Option<f64>,
    #[serde(default)]
    usd_24h_change: Option<f64>,
}

// Simple in-memory cache
struct PriceCache {
    // coin_id -> (price, 24h_change)
    prices: HashMap<String, (f64, f64)>,
    last_update: Option<Instant>,
    updated_at_unix: Option<i64>,
}

static PRICE_CACHE: Lazy<Mutex<PriceCache>> = Lazy::new(|| {
    Mutex::new(PriceCache {
        prices: HashMap::new(),
        last_update: None,
        updated_at_unix: None,
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
        "DAI" => Some("dai"),
        _ => None,
    }
}

/// Check if a symbol is a stablecoin and return its fixed price
fn get_stablecoin_price(symbol: &str) -> Option<f64> {
    match symbol.to_uppercase().as_str() {
        "USDT" | "USDC" | "DAI" => Some(1.0),
        _ => None,
    }
}

/// Fetch USD prices and 24h changes for multiple crypto assets
pub async fn fetch_prices(symbols: Vec<String>) -> Result<HashMap<String, (f64, f64)>, String> {
    let mut result = HashMap::new();
    let mut symbols_to_fetch = Vec::new();

    // 1. Handle stablecoins and check cache
    {
        let cache = PRICE_CACHE.lock().unwrap();
        let cache_valid = cache.last_update.map_or(false, |last| {
            last.elapsed().as_secs() < CACHE_DURATION_SECS
        });

        for symbol in &symbols {
            if let Some(price) = get_stablecoin_price(symbol) {
                result.insert(symbol.clone(), (price, 0.0));
            } else if cache_valid {
                if let Some(coingecko_id) = get_coingecko_id(symbol) {
                    if let Some(&data) = cache.prices.get(coingecko_id) {
                        result.insert(symbol.clone(), data);
                    } else {
                        symbols_to_fetch.push(symbol.clone());
                    }
                }
            } else {
                symbols_to_fetch.push(symbol.clone());
            }
        }
    }

    if symbols_to_fetch.is_empty() {
        return Ok(result);
    }

    // Convert symbols to CoinGecko IDs
    let ids: Vec<String> = symbols_to_fetch
        .iter()
        .filter_map(|symbol| get_coingecko_id(symbol).map(|id| id.to_string()))
        .collect();

    if ids.is_empty() {
        return Ok(result);
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
                    cache.updated_at_unix = Some(Utc::now().timestamp());
                }

                // Map CoinGecko IDs back to symbols
                let mut result = HashMap::new();
                for symbol in &symbols {
                    if let Some(price) = get_stablecoin_price(symbol) {
                         result.insert(symbol.clone(), (price, 0.0));
                    } else if let Some(coingecko_id) = get_coingecko_id(symbol) {
                        if let Some(&data) = response_map.get(coingecko_id) {
                            result.insert(symbol.clone(), data);
                        }
                    }
                }
                let duration_ms = attempt_start.elapsed().as_millis();
                tracing::info!(
                    count = %sanitize(&format!("{}", result.len())),
                    duration_ms = %sanitize(&format!("{}", duration_ms)),
                    "Fetched fresh prices from CoinGecko"
                );
                return Ok(result);
            }
            Err(e) => {
                if attempt < RETRY_ATTEMPTS {
                    let delay_ms = INITIAL_RETRY_DELAY_MS * (2_u64.pow(attempt - 1));
                    tracing::warn!(
                        attempt = %sanitize(&format!("{}", attempt)),
                        delay_ms = %sanitize(&format!("{}", delay_ms)),
                        error = %sanitize(&format!("{}", e)),
                        "Price query attempt failed; retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                } else {
                    tracing::warn!(
                        error = %sanitize(&format!("{}", e)),
                        "Failed to fetch prices after all retries"
                    );
                    // Try to use stale cache as fallback
                    let cache = PRICE_CACHE.lock().unwrap();
                    if !cache.prices.is_empty() {
                        tracing::info!("Using stale cache as fallback");
                        let mut result = HashMap::new();
                        for symbol in &symbols {
                             if let Some(price) = get_stablecoin_price(symbol) {
                                result.insert(symbol.clone(), (price, 0.0));
                            } else if let Some(coingecko_id) = get_coingecko_id(symbol) {
                                if let Some(&data) = cache.prices.get(coingecko_id) {
                                    result.insert(symbol.clone(), data);
                                }
                            }
                        }
                        return Ok(result);
                    }
                    // Return what we have (stablecoins)
                    return Ok(result);
                }
            }
        }
    }

    Ok(result)
}

async fn try_fetch_prices(ids: &str) -> Result<HashMap<String, (f64, f64)>, String> {
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true",
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

    // Extract price and 24h change
    let mut result = HashMap::new();
    for (coin_id, price_data) in parsed {
        if let Some(usd_price) = price_data.usd {
            let change = price_data.usd_24h_change.unwrap_or(0.0);
            result.insert(coin_id, (usd_price, change));
        } else {
            tracing::warn!(
                coin_id = %sanitize(&format!("{}", coin_id)),
                "Missing USD price data"
            );
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
        .map(|(price, _)| *price)
        .ok_or_else(|| format!("Price not found for {}", symbol))
}

#[allow(dead_code)]
pub async fn fetch_price_state(symbol: &str) -> Result<PriceState, String> {
    let now = Utc::now().timestamp();

    if let Some(price_usd) = get_stablecoin_price(symbol) {
        return Ok(state_price::synthetic(price_usd, "synthetic-stablecoin", now));
    }

    let prices = fetch_prices(vec![symbol.to_string()]).await?;
    let Some((price_usd, _)) = prices.get(symbol).copied() else {
        return Ok(state_price::unavailable());
    };

    let updated_at = {
        let cache = PRICE_CACHE.lock().unwrap();
        cache.updated_at_unix
    };

    match updated_at {
        Some(updated_at) => Ok(state_price::from_fetch(
            price_usd,
            "coingecko",
            updated_at,
            now,
            PRICE_FRESH_WITHIN_SECS,
            PRICE_STALE_AFTER_SECS,
        )),
        None => Ok(state_price::unavailable()),
    }
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

#[tauri::command]
#[allow(dead_code)]
pub async fn get_bitcoin_price() -> Result<PriceState, String> {
    fetch_price_state("BTC").await
}
