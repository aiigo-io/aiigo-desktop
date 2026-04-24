use crate::db::AssetBalanceData;
use crate::wallet::bitcoin::balance::BitcoinChainAdapter;
use crate::wallet::bitcoin::transaction as bitcoin_transaction;
use crate::wallet::chain::traits::{ChainAdapter, ChainBalanceSnapshot};
use crate::wallet::evm::balance::EvmChainAdapter;
use crate::wallet::evm::config::{chain_concurrency_limit, get_all_chains};
use crate::wallet::evm::transaction as evm_transaction;
use crate::wallet::state::types::{FreshnessMetadata, FreshnessStatus};
use crate::wallet::sync::types::{SyncOutcome, SyncReason, SyncTarget};
use crate::wallet::transaction_types::{BitcoinTransaction, EvmTransaction, TransactionStatus};
use crate::wallet::types::{EvmAssetBalance, EvmChainAssets, EvmWalletInfo, ValuationStatus, WalletInfo};
use crate::DB;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

#[derive(Debug, Clone, PartialEq)]
pub struct DashboardRefreshResult {
    pub total_balance_usd: f64,
    pub total_balance_btc: f64,
    pub change_24h_amount: f64,
    pub change_24h_percentage: f64,
}

fn asset_valuation(symbol: &str, balance_float: f64) -> (Option<f64>, Option<f64>, ValuationStatus) {
    match crate::wallet::evm::price_manager::get_cached_price_state(symbol).price_usd {
        Some(usd_price) => (Some(usd_price), Some(balance_float * usd_price), ValuationStatus::Valued),
        None => (None, None, ValuationStatus::Unpriced),
    }
}

pub(crate) fn summarize_asset_valuations(assets: &[EvmAssetBalance]) -> (f64, usize, ValuationStatus) {
    let total_balance_usd = assets.iter().filter_map(|asset| asset.usd_value).sum();
    let unpriced_asset_count = assets
        .iter()
        .filter(|asset| matches!(asset.valuation_status, ValuationStatus::Unpriced) && asset.balance_float > 0.0)
        .count();

    (
        total_balance_usd,
        unpriced_asset_count,
        if unpriced_asset_count > 0 {
            ValuationStatus::Unpriced
        } else {
            ValuationStatus::Valued
        },
    )
}

pub async fn sync_bitcoin_wallet_balance(
    wallet_id: &str,
    reason: SyncReason,
) -> Result<(WalletInfo, SyncOutcome), String> {
    let wallet = {
        let db = DB.lock().unwrap();
        db.get_bitcoin_wallet(wallet_id)
            .map_err(|e| format!("Failed to get wallet: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

    let adapter = BitcoinChainAdapter::new(wallet.address.clone());
    let sync_result = adapter.fetch_balances().await;

    let (balance, failed_sources, partial) = match sync_result {
        Ok(snapshot) => (
            snapshot
                .assets
                .first()
                .map(|asset| asset.display_amount)
                .unwrap_or(0.0),
            Vec::new(),
            false,
        ),
        Err(error) => {
            let failed_sources = vec![format!("bitcoin:{}", error)];

            {
                let db = DB.lock().unwrap();
                db.update_bitcoin_wallet_sync_metadata(&wallet.id, &failed_sources)
                    .map_err(|e| format!("Failed to update sync metadata: {}", e))?;
            }

            return Ok((
                wallet.clone(),
                SyncOutcome {
                    reason,
                    target: SyncTarget::BitcoinWalletBalance,
                    updated_at: Some(Utc::now().timestamp()),
                    partial: true,
                    failed_sources,
                },
            ));
        }
    };

    {
        let db = DB.lock().unwrap();
        db.update_bitcoin_wallet_balance(&wallet.id, balance)
            .map_err(|e| format!("Failed to update balance: {}", e))?;
        db.update_bitcoin_wallet_sync_metadata(&wallet.id, &failed_sources)
            .map_err(|e| format!("Failed to update sync metadata: {}", e))?;
    }

    Ok((
        WalletInfo {
            balance,
            ..wallet
        },
        SyncOutcome {
            reason,
            target: SyncTarget::BitcoinWalletBalance,
            updated_at: Some(Utc::now().timestamp()),
            partial,
            failed_sources,
        },
    ))
}

pub async fn sync_evm_wallet_balances(
    wallet_id: &str,
    reason: SyncReason,
) -> Result<(EvmWalletInfo, SyncOutcome), String> {
    let wallet = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet(wallet_id)
            .map_err(|e| format!("Failed to get wallet: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

            let cached_chain_assets = cached_evm_chain_assets(&wallet.id)?;

    let chains_config = get_all_chains();
    let concurrency_limit = chain_concurrency_limit();
    let semaphore = Arc::new(Semaphore::new(concurrency_limit));
    let mut set = JoinSet::new();

    for (order, chain_config) in chains_config.into_iter().enumerate() {
        let semaphore = semaphore.clone();
        let wallet_address = wallet.address.clone();
        let wallet_id = wallet.id.clone();
        let cached_assets = cached_chain_assets
            .get(chain_config.name())
            .cloned()
            .unwrap_or_default();

        set.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .map_err(|e| format!("Semaphore closed: {}", e))?;

            let adapter = EvmChainAdapter::new(chain_config, wallet_address.clone());
            match adapter.fetch_balances().await {
                Ok(snapshot) => Ok::<_, String>((
                    order,
                    chain_snapshot_to_assets(&wallet_id, snapshot),
                    Vec::<String>::new(),
                )),
                Err(error) => Ok((
                    order,
                    failed_chain_assets(chain_config.name(), chain_config.chain_id(), cached_assets),
                    vec![chain_config.name().to_string(), error],
                )),
            }
        });
    }

    let mut chains_with_order = Vec::new();
    let mut all_asset_data = Vec::new();
    let mut total_balance_usd = 0.0;
    let mut failed_sources = Vec::new();

    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok((order, (chain_assets, asset_data), failures))) => {
                if chain_assets.chain_id != 11155111
                    && chain_assets.freshness.status != FreshnessStatus::Unavailable
                {
                    total_balance_usd += chain_assets.total_balance_usd;
                }

                if failures.is_empty() {
                    all_asset_data.extend(asset_data);
                } else {
                    failed_sources.extend(failures.into_iter().take(1));
                }

                chains_with_order.push((order, chain_assets));
            }
            Ok(Err(error)) => failed_sources.push(error),
            Err(join_err) => failed_sources.push(format!("join:{:?}", join_err)),
        }
    }

    chains_with_order.sort_by_key(|(order, _)| *order);
    let chains: Vec<EvmChainAssets> = chains_with_order
        .into_iter()
        .map(|(_, chain)| chain)
        .collect();
    let wallet_unpriced_asset_count = chains
        .iter()
        .filter(|chain| chain.chain_id != 11155111)
        .map(|chain| chain.unpriced_asset_count)
        .sum();

    {
        let db = DB.lock().unwrap();
        if !all_asset_data.is_empty() {
            db.batch_save_evm_asset_balances(&all_asset_data)
                .map_err(|e| format!("Failed to save balances: {}", e))?;
        }
        db.update_evm_wallet_sync_metadata(&wallet.id, total_balance_usd, &failed_sources)
            .map_err(|e| format!("Failed to update wallet metadata: {}", e))?;
    }

    let deduped_failed_sources = dedupe_failed_sources(failed_sources);

    Ok((
        EvmWalletInfo {
            id: wallet.id,
            label: wallet.label,
            wallet_type: wallet.wallet_type,
            address: wallet.address,
            chains,
            total_balance_usd,
            valuation_status: if wallet_unpriced_asset_count > 0 {
                ValuationStatus::Unpriced
            } else {
                ValuationStatus::Valued
            },
            unpriced_asset_count: wallet_unpriced_asset_count,
            created_at: wallet.created_at,
            updated_at: wallet.updated_at,
        },
        SyncOutcome {
            reason,
            target: SyncTarget::EvmWalletBalances,
            updated_at: Some(Utc::now().timestamp()),
            partial: !deduped_failed_sources.is_empty(),
            failed_sources: deduped_failed_sources,
        },
    ))
}

pub async fn refresh_dashboard(
    reason: SyncReason,
) -> Result<(DashboardRefreshResult, SyncOutcome), String> {
    let (bitcoin_wallet_ids, evm_wallet_ids) = {
        let db = DB.lock().map_err(|e| e.to_string())?;
        let bitcoin_wallet_ids = db
            .get_bitcoin_wallets()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|wallet| wallet.id)
            .collect::<Vec<_>>();
        let evm_wallet_ids = db
            .get_evm_wallets()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|wallet| wallet.id)
            .collect::<Vec<_>>();

        (bitcoin_wallet_ids, evm_wallet_ids)
    };

    let mut failed_sources = Vec::new();

    for wallet_id in bitcoin_wallet_ids {
        match sync_bitcoin_wallet_balance(&wallet_id, reason).await {
            Ok((_, outcome)) => failed_sources.extend(outcome.failed_sources),
            Err(error) => failed_sources.push(format!("bitcoin:{}", error)),
        }
    }

    for wallet_id in evm_wallet_ids {
        match sync_evm_wallet_balances(&wallet_id, reason).await {
            Ok((_, outcome)) => failed_sources.extend(outcome.failed_sources),
            Err(error) => failed_sources.push(format!("evm:{}", error)),
        }
    }

    let result = {
        let db = DB.lock().map_err(|e| e.to_string())?;

        let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
        let total_btc_balance: f64 = btc_wallets.iter().map(|wallet| wallet.balance).sum();
        let btc_price_state = crate::wallet::evm::price_manager::get_cached_price_state("BTC");
        let btc_price = btc_price_state.price_usd.unwrap_or(0.0);
        let btc_price_unavailable = total_btc_balance > 0.0
            && matches!(btc_price_state.status, crate::wallet::state::types::PriceStatus::Unavailable);
        if btc_price_unavailable {
            failed_sources.push("price:btc_unavailable".to_string());
        }
        let btc_value_usd = if btc_price_unavailable {
            0.0
        } else {
            total_btc_balance * btc_price
        };

        let evm_wallets = db.get_evm_wallets().map_err(|e| e.to_string())?;
        let mut total_evm_usd = 0.0;
        let mut all_assets = Vec::new();

        for wallet in evm_wallets {
            let assets = db.get_evm_asset_balances(&wallet.id).map_err(|e| e.to_string())?;
            for asset in assets {
                if asset.2 != 11155111 {
                    total_evm_usd += asset.9.unwrap_or(0.0);
                }
                all_assets.push(asset);
            }
        }

        let total_portfolio_usd = btc_value_usd + total_evm_usd;
        let total_portfolio_btc = if btc_price > 0.0 && !btc_price_unavailable {
            total_portfolio_usd / btc_price
        } else {
            0.0
        };

        let btc_24h_change = crate::wallet::evm::price_manager::get_cached_24h_change("BTC").unwrap_or(0.0);
        let btc_change_amount = if btc_price_unavailable {
            0.0
        } else {
            btc_value_usd * (btc_24h_change / 100.0)
        };

        let mut evm_change_amount = 0.0;
        for asset in &all_assets {
            if let (Some(change), Some(usd_value)) = (
                crate::wallet::evm::price_manager::get_cached_24h_change(&asset.1),
                asset.9,
            ) {
                evm_change_amount += usd_value * (change / 100.0);
            }
        }

        let total_change_amount = btc_change_amount + evm_change_amount;
        let total_change_percentage = if total_portfolio_usd > 0.0 {
            (total_change_amount / total_portfolio_usd) * 100.0
        } else {
            0.0
        };

        db.update_dashboard_stats(
            total_portfolio_usd,
            total_portfolio_btc,
            total_change_amount,
            total_change_percentage,
            &failed_sources,
        )
        .map_err(|e| e.to_string())?;
        if !btc_price_unavailable {
            db.save_portfolio_snapshot(total_portfolio_usd)
                .map_err(|e| e.to_string())?;
        }

        DashboardRefreshResult {
            total_balance_usd: total_portfolio_usd,
            total_balance_btc: total_portfolio_btc,
            change_24h_amount: total_change_amount,
            change_24h_percentage: total_change_percentage,
        }
    };

    let deduped_failed_sources = dedupe_failed_sources(failed_sources);

    Ok((
        result,
        SyncOutcome {
            reason,
            target: SyncTarget::Dashboard,
            updated_at: Some(Utc::now().timestamp()),
            partial: !deduped_failed_sources.is_empty(),
            failed_sources: deduped_failed_sources,
        },
    ))
}

pub async fn refresh_bitcoin_history(
    wallet_id: String,
    address: String,
    reason: SyncReason,
) -> Result<(Vec<BitcoinTransaction>, SyncOutcome), String> {
    let transactions = bitcoin_transaction::fetch_bitcoin_transaction_history(wallet_id, address).await?;

    Ok((
        transactions,
        SyncOutcome {
            reason,
            target: SyncTarget::BitcoinHistory,
            updated_at: Some(Utc::now().timestamp()),
            partial: false,
            failed_sources: Vec::new(),
        },
    ))
}

pub async fn refresh_evm_history(
    wallet_id: String,
    address: String,
    chain: String,
    chain_id: u64,
    reason: SyncReason,
) -> Result<(Vec<EvmTransaction>, SyncOutcome), String> {
    let transactions =
        evm_transaction::fetch_evm_transaction_history(wallet_id, address, chain, chain_id).await?;

    Ok((
        transactions,
        SyncOutcome {
            reason,
            target: SyncTarget::EvmHistory,
            updated_at: Some(Utc::now().timestamp()),
            partial: false,
            failed_sources: Vec::new(),
        },
    ))
}

pub async fn refresh_evm_transaction_receipt_status(
    tx_hash: String,
    chain_id: u64,
    reason: SyncReason,
) -> Result<(TransactionStatus, SyncOutcome), String> {
    let receipt = evm_transaction::get_transaction_receipt(tx_hash, chain_id).await?;
    let status = match receipt {
        Some(receipt) => TransactionStatus::from_evm_receipt(
            receipt.status.map(|value| value.as_u64() == 1),
            Some(crate::wallet::sync::types::EVM_MIN_BLOCK_DEPTH),
            crate::wallet::sync::types::EVM_MIN_BLOCK_DEPTH,
        ),
        None => TransactionStatus::Pending,
    };

    Ok((
        status,
        SyncOutcome {
            reason,
            target: SyncTarget::TransactionLifecycle,
            updated_at: Some(Utc::now().timestamp()),
            partial: false,
            failed_sources: Vec::new(),
        },
    ))
}

fn chain_snapshot_to_assets(
    wallet_id: &str,
    snapshot: ChainBalanceSnapshot,
) -> (EvmChainAssets, Vec<AssetBalanceData>) {
    let updated_at = Utc::now().timestamp();
    let chain_id = snapshot
        .chain_id
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_default();
    let mut chain_total_usd = 0.0;
    let mut persisted_assets = Vec::new();
    let mut assets = Vec::new();
    let mut unpriced_asset_count = 0;

    for asset in snapshot.assets {
        let (usd_price, usd_value, valuation_status) = asset_valuation(&asset.symbol, asset.display_amount);
        if let Some(usd_value) = usd_value {
            chain_total_usd += usd_value;
        } else if asset.display_amount > 0.0 {
            unpriced_asset_count += 1;
        }

        persisted_assets.push(AssetBalanceData {
            wallet_id: wallet_id.to_string(),
            chain: snapshot.chain_name.clone(),
            chain_id,
            symbol: asset.symbol.clone(),
            name: asset.name.clone(),
            decimals: asset.decimals,
            contract_address: asset.contract_address.clone(),
            balance: asset.raw_amount.clone(),
            balance_float: asset.display_amount,
            usd_price,
            usd_value,
            valuation_status: match valuation_status {
                ValuationStatus::Valued => "valued".to_string(),
                ValuationStatus::Unpriced => "unpriced".to_string(),
            },
        });

        assets.push(EvmAssetBalance {
            chain: snapshot.chain_name.clone(),
            asset: crate::wallet::types::EvmAsset {
                symbol: asset.symbol,
                name: asset.name,
                decimals: asset.decimals,
                contract_address: asset.contract_address,
            },
            balance: asset.raw_amount,
            balance_float: asset.display_amount,
            usd_price,
            usd_value,
            valuation_status,
        });
    }

    (
        EvmChainAssets {
            chain: snapshot.chain_name,
            chain_id,
            total_balance_usd: chain_total_usd,
            valuation_status: if unpriced_asset_count > 0 {
                ValuationStatus::Unpriced
            } else {
                ValuationStatus::Valued
            },
            unpriced_asset_count,
            freshness: FreshnessMetadata {
                status: FreshnessStatus::Fresh,
                updated_at: Some(updated_at),
                failed_sources: Vec::new(),
            },
            assets,
        },
        persisted_assets,
    )
}

pub(crate) fn failed_chain_freshness(
    chain_name: &str,
    has_cached_assets: bool,
    updated_at: Option<i64>,
) -> FreshnessMetadata {
    let status = if has_cached_assets {
        FreshnessStatus::Partial
    } else {
        FreshnessStatus::Unavailable
    };

    FreshnessMetadata {
        status,
        updated_at,
        failed_sources: vec![chain_name.to_string()],
    }
}

fn failed_chain_assets(
    chain_name: &str,
    chain_id: u64,
    assets: Vec<EvmAssetBalance>,
) -> (EvmChainAssets, Vec<AssetBalanceData>) {
    let (total_balance_usd, unpriced_asset_count, valuation_status) = summarize_asset_valuations(&assets);

    (
        EvmChainAssets {
            chain: chain_name.to_string(),
            chain_id,
            total_balance_usd,
            valuation_status,
            unpriced_asset_count,
            freshness: failed_chain_freshness(chain_name, !assets.is_empty(), None),
            assets,
        },
        Vec::new(),
    )
}

fn cached_evm_chain_assets(
    wallet_id: &str,
) -> Result<HashMap<String, Vec<EvmAssetBalance>>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let rows = db.get_evm_asset_balances(wallet_id).map_err(|e| e.to_string())?;
    let mut chains = HashMap::new();

    for (chain, symbol, _chain_id, name, contract_address, decimals, balance, balance_float, usd_price, usd_value, valuation_status) in rows {
        chains.entry(chain.clone()).or_insert_with(Vec::new).push(EvmAssetBalance {
            chain,
            asset: crate::wallet::types::EvmAsset {
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

    Ok(chains)
}

fn dedupe_failed_sources(failed_sources: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();

    for source in failed_sources {
        if !deduped.contains(&source) {
            deduped.push(source);
        }
    }

    deduped
}

#[cfg(test)]
mod tests {
    use super::{dedupe_failed_sources, failed_chain_assets, failed_chain_freshness, summarize_asset_valuations};
    use crate::wallet::state::types::FreshnessStatus;
    use crate::wallet::types::{EvmAsset, EvmAssetBalance, ValuationStatus};

    #[test]
    fn sync_engine_dedupes_failed_sources_without_reordering() {
        assert_eq!(
            dedupe_failed_sources(vec![
                "ethereum".to_string(),
                "arbitrum".to_string(),
                "ethereum".to_string(),
            ]),
            vec!["ethereum".to_string(), "arbitrum".to_string()]
        );
    }

    #[test]
    fn failed_chain_assets_marks_cached_chain_as_partial() {
        let (chain, _) = failed_chain_assets(
            "ethereum",
            1,
            vec![EvmAssetBalance {
                chain: "ethereum".to_string(),
                asset: EvmAsset::new("ETH", "Ethereum", 18, None),
                balance: "1".to_string(),
                balance_float: 1.0,
                usd_price: Some(10.0),
                usd_value: Some(10.0),
                valuation_status: ValuationStatus::Valued,
            }],
        );

        assert_eq!(chain.freshness.status, FreshnessStatus::Partial);
        assert_eq!(chain.total_balance_usd, 10.0);
    }

    #[test]
    fn failed_chain_assets_marks_uncached_chain_as_unavailable() {
        let (chain, _) = failed_chain_assets("arbitrum", 42161, Vec::new());

        assert_eq!(chain.freshness.status, FreshnessStatus::Unavailable);
        assert_eq!(chain.total_balance_usd, 0.0);
    }

    #[test]
    fn failed_chain_freshness_keeps_last_success_timestamp_for_cached_partial_chain() {
        let freshness = failed_chain_freshness("ethereum", true, Some(1_713_499_200));

        assert_eq!(freshness.status, FreshnessStatus::Partial);
        assert_eq!(freshness.updated_at, Some(1_713_499_200));
        assert_eq!(freshness.failed_sources, vec!["ethereum".to_string()]);
    }

    #[test]
    fn summarize_asset_valuations_ignores_unpriced_assets_from_subtotal() {
        let (subtotal, unpriced_count, status) = summarize_asset_valuations(&[
            EvmAssetBalance {
                chain: "ethereum".to_string(),
                asset: EvmAsset::new("ETH", "Ethereum", 18, None),
                balance: "1".to_string(),
                balance_float: 1.0,
                usd_price: Some(10.0),
                usd_value: Some(10.0),
                valuation_status: ValuationStatus::Valued,
            },
            EvmAssetBalance {
                chain: "ethereum".to_string(),
                asset: EvmAsset::new("ABC", "ABC", 18, Some("0xdef")),
                balance: "2".to_string(),
                balance_float: 2.0,
                usd_price: None,
                usd_value: None,
                valuation_status: ValuationStatus::Unpriced,
            },
        ]);

        assert_eq!(subtotal, 10.0);
        assert_eq!(unpriced_count, 1);
        assert_eq!(status, ValuationStatus::Unpriced);
    }
}