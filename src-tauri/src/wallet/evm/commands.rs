use crate::wallet::sync::engine;
use crate::wallet::sync::types::SyncReason;
use crate::wallet::types::{EvmAsset, EvmAssetBalance, EvmChainAssets, EvmWalletBalancesResponse, EvmWalletInfo, ValuationStatus, WalletInfo};
use crate::wallet::state::types::{FreshnessMetadata, FreshnessStatus};
use crate::DB;

const BALANCE_FRESH_WITHIN_SECS: i64 = 60;
const BALANCE_STALE_AFTER_SECS: i64 = 300;

fn load_wallet_level_freshness(wallet_id: &str) -> Result<FreshnessMetadata, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    db.get_evm_wallet_balance_freshness(
        wallet_id,
        chrono::Utc::now().timestamp(),
        BALANCE_FRESH_WITHIN_SECS,
        BALANCE_STALE_AFTER_SECS,
    )
    .map_err(|e| format!("Failed to get EVM wallet freshness: {}", e))?
    .ok_or_else(|| "EVM wallet freshness not found".to_string())
}

fn query_sync_outcome(wallet_freshness: &FreshnessMetadata) -> crate::wallet::sync::types::SyncOutcome {
    crate::wallet::sync::types::SyncOutcome {
        reason: SyncReason::Query,
        target: crate::wallet::sync::types::SyncTarget::EvmWalletBalances,
        updated_at: wallet_freshness.updated_at,
        partial: !wallet_freshness.failed_sources.is_empty(),
        failed_sources: wallet_freshness.failed_sources.clone(),
    }
}

fn query_chain_freshness(
    chain_name: &str,
    assets: &[EvmAssetBalance],
    wallet_freshness: &FreshnessMetadata,
) -> FreshnessMetadata {
    let has_failure = wallet_freshness
        .failed_sources
        .iter()
        .any(|source| source == chain_name);

    if has_failure {
        return engine::failed_chain_freshness(chain_name, !assets.is_empty(), wallet_freshness.updated_at);
    }

    if assets.is_empty() {
        return FreshnessMetadata {
            status: FreshnessStatus::Unavailable,
            updated_at: None,
            failed_sources: Vec::new(),
        };
    }

    FreshnessMetadata {
        status: wallet_freshness.status.clone(),
        updated_at: wallet_freshness.updated_at,
        failed_sources: Vec::new(),
    }
}

fn query_evm_wallet_balances_inner(wallet_id: &str) -> Result<EvmWalletBalancesResponse, String> {
    let wallet = {
        let db = DB.lock().map_err(|e| e.to_string())?;
        db.get_evm_wallet(wallet_id)
            .map_err(|e| format!("Failed to get wallet: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

    let wallet_freshness = load_wallet_level_freshness(wallet_id)?;
    let rows = {
        let db = DB.lock().map_err(|e| e.to_string())?;
        db.get_evm_asset_balances(wallet_id)
            .map_err(|e| format!("Failed to get cached balances: {}", e))?
    };

    let mut chains = std::collections::BTreeMap::<(String, u64), Vec<EvmAssetBalance>>::new();
    for (chain, symbol, chain_id, name, contract_address, decimals, balance, balance_float, usd_price, usd_value, valuation_status) in rows {
        chains
            .entry((chain.clone(), chain_id))
            .or_default()
            .push(EvmAssetBalance {
                chain,
                asset: EvmAsset {
                    symbol,
                    name,
                    decimals,
                    contract_address,
                },
                balance,
                balance_float,
                usd_price,
                usd_value,
                valuation_status: if valuation_status == "unpriced" {
                    ValuationStatus::Unpriced
                } else {
                    ValuationStatus::Valued
                },
            });
    }

    let supported_chains = crate::wallet::evm::config::get_all_chains();
    let chain_views = supported_chains
        .into_iter()
        .map(|chain_config| {
            let key = (chain_config.name().to_string(), chain_config.chain_id());
            let assets = chains.remove(&key).unwrap_or_default();
            let freshness = query_chain_freshness(chain_config.name(), &assets, &wallet_freshness);
            let (total_balance_usd, unpriced_asset_count, valuation_status) = engine::summarize_asset_valuations(&assets);

            EvmChainAssets {
                chain: chain_config.name().to_string(),
                chain_id: chain_config.chain_id(),
                total_balance_usd,
                valuation_status,
                unpriced_asset_count,
                freshness,
                assets,
            }
        })
        .collect::<Vec<_>>();

    let total_balance_usd = chain_views
        .iter()
        .filter(|chain| chain.chain_id != 11155111)
        .map(|chain| chain.total_balance_usd)
        .sum();
    let unpriced_asset_count = chain_views
        .iter()
        .filter(|chain| chain.chain_id != 11155111)
        .map(|chain| chain.unpriced_asset_count)
        .sum();

    Ok(EvmWalletBalancesResponse {
        wallet: EvmWalletInfo {
            id: wallet.id,
            label: wallet.label,
            wallet_type: wallet.wallet_type,
            address: wallet.address,
            total_balance_usd,
            valuation_status: if unpriced_asset_count > 0 {
                ValuationStatus::Unpriced
            } else {
                ValuationStatus::Valued
            },
            unpriced_asset_count,
            chains: chain_views,
            created_at: wallet.created_at,
            updated_at: wallet.updated_at,
        },
        sync: query_sync_outcome(&wallet_freshness),
    })
}

#[tauri::command]
pub fn evm_get_wallets() -> Result<Vec<WalletInfo>, String> {
    let db = DB.lock().unwrap();
    db.get_evm_wallets()
        .map_err(|e| format!("Failed to get wallets: {}", e))
}

#[tauri::command]
pub fn evm_get_wallet(wallet_id: String) -> Result<Option<WalletInfo>, String> {
    let db = DB.lock().unwrap();
    db.get_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))
}

#[tauri::command]
pub async fn evm_get_wallet_with_balances(wallet_id: String) -> Result<EvmWalletBalancesResponse, String> {
    refresh_evm_wallet_balances(wallet_id).await
}

#[tauri::command]
pub fn query_evm_wallet_balances(wallet_id: String) -> Result<EvmWalletBalancesResponse, String> {
    query_evm_wallet_balances_inner(&wallet_id)
}

#[tauri::command]
pub async fn refresh_evm_wallet_balances(wallet_id: String) -> Result<EvmWalletBalancesResponse, String> {
    engine::sync_evm_wallet_balances(&wallet_id, SyncReason::Manual)
        .await
        .map(|(wallet, sync)| EvmWalletBalancesResponse { wallet, sync })
}

#[cfg(test)]
mod tests {
    use super::{evm_get_wallet_with_balances, query_chain_freshness, query_sync_outcome};
    use crate::wallet::state::types::{FreshnessMetadata, FreshnessStatus};
    use crate::wallet::sync::types::SyncReason;
    use crate::wallet::types::{EvmAsset, EvmAssetBalance, EvmWalletBalancesResponse, ValuationStatus};
    use std::future::Future;

    fn assert_command_shape<F, Fut>(_command: F)
    where
        F: Fn(String) -> Fut,
        Fut: Future<Output = Result<EvmWalletBalancesResponse, String>>,
    {
    }

    #[test]
    fn evm_wallet_with_balances_command_uses_typed_partial_failure_response() {
        assert_command_shape(evm_get_wallet_with_balances);
    }

    #[test]
    fn query_sync_outcome_keeps_missing_timestamp_honest() {
        let outcome = query_sync_outcome(&FreshnessMetadata {
            status: FreshnessStatus::Unavailable,
            updated_at: None,
            failed_sources: Vec::new(),
        });

        assert_eq!(outcome.reason, SyncReason::Query);
        assert_eq!(outcome.updated_at, None);
        assert!(!outcome.partial);
    }

    #[test]
    fn query_chain_freshness_preserves_partial_for_failed_cached_chain() {
        let freshness = query_chain_freshness(
            "ethereum",
            &[EvmAssetBalance {
                chain: "ethereum".to_string(),
                asset: EvmAsset::new("ETH", "Ethereum", 18, None),
                balance: "1".to_string(),
                balance_float: 1.0,
                usd_price: Some(10.0),
                usd_value: Some(10.0),
                valuation_status: ValuationStatus::Valued,
            }],
            &FreshnessMetadata {
                status: FreshnessStatus::Fresh,
                updated_at: Some(1_713_499_200),
                failed_sources: vec!["ethereum".to_string()],
            },
        );

        assert_eq!(freshness.status, FreshnessStatus::Partial);
        assert_eq!(freshness.updated_at, Some(1_713_499_200));
        assert_eq!(freshness.failed_sources, vec!["ethereum".to_string()]);
    }
}

#[tauri::command]
pub fn evm_delete_wallet(wallet_id: String) -> Result<bool, String> {
    let db = DB.lock().unwrap();
    db.delete_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to delete wallet: {}", e))
}
