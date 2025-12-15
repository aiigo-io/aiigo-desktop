use crate::wallet::evm::config::EvmChainConfig;
use crate::wallet::evm::provider::{HybridProvider, ProviderError, ProviderRegistry};
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

const RETRY_ATTEMPTS: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 500;

/// Get all balances for a wallet on a specific chain.
pub async fn get_chain_balances(
    chain_config: EvmChainConfig,
    wallet_address: &str,
) -> Result<Vec<(String, String, f64)>, String> {
    let wallet_addr: Address = wallet_address
        .parse()
        .map_err(|_| "Invalid wallet address".to_string())?;

    let provider = ProviderRegistry::get_or_init(chain_config)
        .await
        .map_err(|e| {
            format!(
                "Failed to initialize provider for {}: {}",
                chain_config.name(),
                e
            )
        })?;

    let mut set = JoinSet::new();

    for asset in chain_config.assets() {
        let provider = provider.clone();
        let symbol = asset.symbol.clone();
        let decimals = asset.decimals;
        if let Some(contract_address) = asset.contract_address.clone() {
            set.spawn(async move {
                let token_addr: Address = contract_address
                    .parse()
                    .map_err(|_| format!("Invalid token address: {}", contract_address))?;

                query_erc20_with_retry(provider, token_addr, wallet_addr, decimals)
                    .await
                    .map(|(balance_str, balance_float)| (symbol, balance_str, balance_float))
            });
        } else {
            set.spawn(async move {
                query_native_with_retry(provider, wallet_addr, decimals)
                    .await
                    .map(|(balance_str, balance_float)| (symbol, balance_str, balance_float))
            });
        }
    }

    let mut balances = Vec::new();
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(tuple)) => balances.push(tuple),
            Ok(Err(err)) => tracing::warn!(chain=%chain_config.name(), error=%err, "Asset balance query failed"),
            Err(join_err) => tracing::error!(chain=%chain_config.name(), join_err=?join_err, "Asset balance task panicked"),
        }
    }

    Ok(balances)
}

async fn query_native_with_retry(
    provider: Arc<HybridProvider>,
    wallet_addr: Address,
    decimals: u8,
) -> Result<(String, f64), String> {
    for attempt in 1..=RETRY_ATTEMPTS {
        match provider.get_balance(wallet_addr).await {
            Ok(balance) => return Ok(convert_balance(balance, decimals)),
            Err(err) => handle_retry("native", attempt, err).await?,
        }
    }

    Err("Failed to query native balance after retries".to_string())
}

async fn query_erc20_with_retry(
    provider: Arc<HybridProvider>,
    token_addr: Address,
    wallet_addr: Address,
    decimals: u8,
) -> Result<(String, f64), String> {
    for attempt in 1..=RETRY_ATTEMPTS {
        let call_data = encode_balance_of_call(wallet_addr);
        let tx: TypedTransaction = TransactionRequest::new()
            .to(token_addr)
            .data(call_data)
            .into();

        match provider.call_contract(&tx).await {
            Ok(result) => {
                let balance = U256::from_big_endian(result.as_ref());
                return Ok(convert_balance(balance, decimals));
            }
            Err(err) => handle_retry("erc20", attempt, err).await?,
        }
    }

    Err("Failed to query ERC20 balance after retries".to_string())
}

async fn handle_retry(label: &str, attempt: u32, err: ProviderError) -> Result<(), String> {
    if attempt >= RETRY_ATTEMPTS {
        return Err(format!("{} balance query failed: {}", label, err));
    }

    let delay_ms = INITIAL_RETRY_DELAY_MS * (2_u64.pow(attempt - 1));
    tracing::warn!(label=%label, attempt=%attempt, delay_ms=%delay_ms, error=%err.to_string(), "Retrying balance query");
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    Ok(())
}

/// Encode a balanceOf(address) call for ERC20 tokens.
fn encode_balance_of_call(address: Address) -> Bytes {
    let mut data = vec![0x70, 0xa0, 0x82, 0x31];
    let mut addr_bytes = [0u8; 32];
    addr_bytes[12..].copy_from_slice(&address.to_fixed_bytes());
    data.extend_from_slice(&addr_bytes);
    Bytes::from(data)
}

fn convert_balance(balance: U256, decimals: u8) -> (String, f64) {
    let balance_str = balance.to_string();

    if decimals == 0 {
        return (balance_str, balance.as_u64() as f64);
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    if divisor.is_zero() {
        return (balance_str, 0.0);
    }

    let whole = balance / divisor;
    let remainder = balance % divisor;

    let remainder_divisor = divisor.as_u64();
    let remainder_f64 = if remainder_divisor > 0 {
        remainder.as_u64() as f64 / remainder_divisor as f64
    } else {
        0.0
    };

    let balance_float = whole.as_u64() as f64 + remainder_f64;
    (balance_str, balance_float)
}
