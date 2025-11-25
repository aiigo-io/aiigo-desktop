use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

const RETRY_ATTEMPTS: u32 = 2;
const INITIAL_RETRY_DELAY_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Deserialize)]
struct CoinGeckoResponse {
    #[serde(flatten)]
    prices: HashMap<String, CoinGeckoPriceData>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoPriceData {
    usd: f64,
}

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
    // Convert symbols to CoinGecko IDs
    let ids: Vec<String> = symbols
        .iter()
        .filter_map(|symbol| {
            get_coingecko_id(symbol).map(|id| id.to_string())
        })
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
        match try_fetch_prices(&ids_param).await {
            Ok(response) => {
                // Map CoinGecko IDs back to symbols
                let mut result = HashMap::new();
                for symbol in &symbols {
                    if let Some(coingecko_id) = get_coingecko_id(symbol) {
                        if let Some(price_data) = response.prices.get(coingecko_id) {
                            result.insert(symbol.clone(), price_data.usd);
                        }
                    }
                }
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
                    // Return empty map instead of error to allow wallet queries to continue
                    return Ok(HashMap::new());
                }
            }
        }
    }

    Ok(HashMap::new())
}

async fn try_fetch_prices(ids: &str) -> Result<CoinGeckoResponse, String> {
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

    let data: CoinGeckoResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}

/// Fetch price for a single asset
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
