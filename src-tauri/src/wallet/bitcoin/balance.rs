use serde::Deserialize;
use std::time::Duration;

const RETRY_ATTEMPTS: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Deserialize)]
struct BlockchainInfoResponse {
    final_balance: u64,
    n_tx: u64,
    total_received: u64,
}

#[derive(Debug, Deserialize)]
struct BlockstreamResponse {
    address: String,
    chain_stats: ChainStats,
    mempool_stats: MempoolStats,
}

#[derive(Debug, Deserialize)]
struct ChainStats {
    funded_txo_sum: u64,
    spent_txo_sum: u64,
}

#[derive(Debug, Deserialize)]
struct MempoolStats {
    funded_txo_sum: u64,
    spent_txo_sum: u64,
}

/// Query BTC balance for an address using blockchain APIs
pub async fn query_btc_balance(address: &str) -> Result<f64, String> {
    // Try multiple blockchain explorer APIs
    let apis = vec![
        (
            "Blockstream",
            format!("https://blockstream.info/api/address/{}", address),
        ),
        (
            "Blockchain.info",
            format!("https://blockchain.info/rawaddr/{}", address),
        ),
    ];

    for (api_name, url) in &apis {
        for attempt in 1..=RETRY_ATTEMPTS {
            match try_query_from_api(api_name, url).await {
                Ok(balance) => {
                    tracing::info!(api=%api_name, balance=%balance, "Retrieved BTC balance");
                    return Ok(balance);
                }
                Err(e) => {
                    if attempt < RETRY_ATTEMPTS {
                        let delay_ms = INITIAL_RETRY_DELAY_MS * (2_u64.pow(attempt - 1));
                        tracing::warn!(attempt=%attempt, api=%api_name, delay_ms=%delay_ms, error=%e.to_string(), "Retrying BTC balance query");
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    } else {
                        tracing::warn!(api=%api_name, "All attempts failed; trying next API");
                    }
                }
            }
        }
    }

    Err("Failed to query balance from all blockchain APIs".to_string())
}

async fn try_query_from_api(api_name: &str, url: &str) -> Result<f64, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    match api_name {
        "Blockstream" => {
            let data: BlockstreamResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Calculate balance: (funded - spent) for both confirmed and mempool
            let confirmed_balance =
                data.chain_stats.funded_txo_sum as i64 - data.chain_stats.spent_txo_sum as i64;
            let mempool_balance =
                data.mempool_stats.funded_txo_sum as i64 - data.mempool_stats.spent_txo_sum as i64;

            let total_satoshis = confirmed_balance + mempool_balance;
            let btc_balance = total_satoshis as f64 / 100_000_000.0;

            Ok(btc_balance)
        }
        "Blockchain.info" => {
            let data: BlockchainInfoResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // final_balance is in satoshis
            let btc_balance = data.final_balance as f64 / 100_000_000.0;

            Ok(btc_balance)
        }
        _ => Err(format!("Unknown API: {}", api_name)),
    }
}
