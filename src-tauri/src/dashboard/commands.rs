use crate::DB;
use crate::dashboard::valuation::build_portfolio_valuation_snapshot;
use crate::wallet::state::freshness::classify_age;
use crate::wallet::state::types::{FreshnessMetadata, FreshnessStatus};
use crate::wallet::sync::engine;
use crate::wallet::sync::types::SyncReason;
use crate::wallet::types::ValuationStatus;
use serde::{Deserialize, Serialize};

const DASHBOARD_FRESH_WITHIN_SECS: i64 = 60;
const DASHBOARD_STALE_AFTER_SECS: i64 = 300;

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_balance_usd: String,
    pub total_balance_btc: String,
    pub change_24h_amount: String,
    pub change_24h_percentage: String,
    pub freshness: FreshnessMetadata,
    pub valuation_status: ValuationStatus,
    pub unpriced_asset_count: usize,
}

// Helper function to format numbers with thousand separators
fn format_currency(value: f64) -> String {
    let abs_value = value.abs();
    let formatted = format!("{:.2}", abs_value);
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = if parts.len() > 1 { parts[1] } else { "00" };
    
    // Add thousand separators
    let mut result = String::new();
    for (i, c) in integer_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    
    let formatted_integer: String = result.chars().rev().collect();
    format!("{}.{}", formatted_integer, decimal_part)
}

fn dashboard_freshness_from_row_with_failures(updated_at: &str, failed_sources_json: &str) -> FreshnessMetadata {
    let updated_at_unix = chrono::DateTime::parse_from_rfc3339(updated_at)
        .map(|dt| dt.timestamp())
        .ok();
    let failed_sources = serde_json::from_str::<Vec<String>>(failed_sources_json).unwrap_or_default();

    let now = chrono::Utc::now().timestamp();
    let age_status = match updated_at_unix {
        Some(updated_at) => classify_age(
            Some(updated_at),
            now,
            DASHBOARD_FRESH_WITHIN_SECS,
            DASHBOARD_STALE_AFTER_SECS,
        ),
        None => FreshnessStatus::Cached,
    };

    let status = if !failed_sources.is_empty() && matches!(age_status, FreshnessStatus::Fresh | FreshnessStatus::Cached) {
        FreshnessStatus::Partial
    } else {
        age_status
    };

    FreshnessMetadata {
        status,
        updated_at: updated_at_unix,
        failed_sources,
    }
}

fn format_dashboard_stats(
    total_balance_usd: f64,
    total_balance_btc: f64,
    change_24h_amount: f64,
    change_24h_percentage: f64,
    freshness: FreshnessMetadata,
    valuation_status: ValuationStatus,
    unpriced_asset_count: usize,
) -> DashboardStats {
    let price_unavailable = freshness
        .failed_sources
        .iter()
        .any(|source| source == "price:btc_unavailable");
    let has_priced_subtotal = total_balance_usd > 0.0;
    let hide_totals_for_unpriced = matches!(valuation_status, ValuationStatus::Unpriced) && !has_priced_subtotal;

    DashboardStats {
        total_balance_usd: if price_unavailable || hide_totals_for_unpriced {
            "$--".to_string()
        } else {
            format!("${}", format_currency(total_balance_usd))
        },
        total_balance_btc: if price_unavailable || hide_totals_for_unpriced {
            "≈ -- BTC".to_string()
        } else {
            format!("≈ {:.4} BTC", total_balance_btc)
        },
        change_24h_amount: if price_unavailable || hide_totals_for_unpriced {
            "--".to_string()
        } else {
            format!("{}${}", if change_24h_amount >= 0.0 { "+" } else { "" }, format_currency(change_24h_amount))
        },
        change_24h_percentage: if price_unavailable || hide_totals_for_unpriced {
            "--".to_string()
        } else {
            format!("{}{:.2}%", if change_24h_percentage >= 0.0 { "+" } else { "" }, change_24h_percentage)
        },
        freshness,
        valuation_status,
        unpriced_asset_count,
    }
}

#[tauri::command]
pub fn get_dashboard_stats() -> Result<DashboardStats, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let stats = db.get_dashboard_stats().map_err(|e| e.to_string())?;
    let valuation_snapshot = build_portfolio_valuation_snapshot(&db)?;

    if let Some((usd, btc, change_amt, change_pct, updated_at, failed_sources_json)) = stats {
        let freshness = dashboard_freshness_from_row_with_failures(&updated_at, &failed_sources_json);
        Ok(format_dashboard_stats(
            usd,
            btc,
            change_amt,
            change_pct,
            freshness,
            valuation_snapshot.valuation_status(),
            valuation_snapshot.unpriced_asset_count,
        ))
    } else {
        Ok(format_dashboard_stats(
            0.0,
            0.0,
            0.0,
            0.0,
            FreshnessMetadata {
                status: FreshnessStatus::Unavailable,
                updated_at: None,
                failed_sources: Vec::new(),
            },
            valuation_snapshot.valuation_status(),
            valuation_snapshot.unpriced_asset_count,
        ))
    }
}

#[tauri::command]
pub async fn refresh_dashboard_stats() -> Result<DashboardStats, String> {
    let _ = engine::refresh_dashboard(SyncReason::Manual).await?;
    get_dashboard_stats()
}

#[derive(Debug, Serialize)]
pub struct PortfolioHistoryPoint {
    pub date: String,
    pub value: f64,
}

#[tauri::command]
pub fn get_portfolio_history() -> Result<Vec<PortfolioHistoryPoint>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let history = db.get_portfolio_history(7).map_err(|e| e.to_string())?;
    
    let result = history.into_iter().map(|(date, value)| {
        PortfolioHistoryPoint { date, value }
    }).collect();
    
    Ok(result)
}

#[derive(Debug, Serialize)]
pub struct AssetAllocation {
    pub name: String,
    pub symbol: String,
    pub percentage: f64,
    pub value_usd: f64,
    pub color: String,
    pub valuation_status: ValuationStatus,
}

#[tauri::command]
pub async fn get_asset_allocation() -> Result<Vec<AssetAllocation>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let snapshot = build_portfolio_valuation_snapshot(&db)?;
    let priced_total_usd = snapshot.priced_total_usd;

    Ok(snapshot
        .allocations
        .into_iter()
        .map(|allocation| AssetAllocation {
            percentage: if matches!(allocation.valuation_status, ValuationStatus::Valued) && priced_total_usd > 0.0 {
                (allocation.value_usd / priced_total_usd) * 100.0
            } else {
                0.0
            },
            name: allocation.name,
            symbol: allocation.symbol,
            value_usd: allocation.value_usd,
            color: allocation.color,
            valuation_status: allocation.valuation_status,
        })
        .collect())
}


#[derive(Debug, Serialize)]
pub struct UnifiedTransaction {
    pub id: String,
    pub r#type: String, // "bitcoin" or "evm"
    pub tx_type: String, // "send", "receive", etc.
    pub status: String,
    pub tx_hash: String,
    pub asset_symbol: String,
    pub amount: String,
    pub timestamp: String,
}

#[tauri::command]
pub fn get_unified_recent_transactions() -> Result<Vec<UnifiedTransaction>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;

    // 1. Get Bitcoin Transactions
    let btc_txs = db.get_all_bitcoin_transactions().map_err(|e| e.to_string())?;
    let mut unified: Vec<UnifiedTransaction> = btc_txs
        .into_iter()
        .map(|tx| UnifiedTransaction {
            id: tx.id,
            r#type: "bitcoin".to_string(),
            tx_type: tx.tx_type.as_str().to_string(),
            status: tx.status.as_str().to_string(),
            tx_hash: tx.tx_hash,
            asset_symbol: "BTC".to_string(),
            amount: format!("{:.8}", tx.amount),
            timestamp: tx.timestamp,
        })
        .collect();

    // 2. Get EVM Transactions
    let evm_txs = db.get_all_evm_transactions().map_err(|e| e.to_string())?;
    let evm_unified: Vec<UnifiedTransaction> = evm_txs
        .into_iter()
        .filter(|tx| tx.chain_id != 11155111) // Filter out Sepolia
        .map(|tx| UnifiedTransaction {
            id: tx.id,
            r#type: "evm".to_string(),
            tx_type: tx.tx_type.as_str().to_string(),
            status: tx.status.as_str().to_string(),
            tx_hash: tx.tx_hash,
            asset_symbol: tx.asset_symbol,
            amount: format!("{:.6}", tx.amount_float),
            timestamp: tx.timestamp,
        })
        .collect();

    unified.extend(evm_unified);

    // 3. Sort by timestamp descending
    unified.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // 4. Return top 5
    if unified.len() > 5 {
        unified.truncate(5);
    }

    Ok(unified)
}

#[derive(Debug, Serialize)]
pub struct UnifiedAsset {
    pub name: String,
    pub symbol: String,
    pub balance: f64,
    pub usd_value: f64,
    pub chain: String, // "Bitcoin" or EVM chain name
    pub is_testnet: bool,
}

#[tauri::command]
pub fn get_unified_assets() -> Result<Vec<UnifiedAsset>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();

    // 1. BTC Wallets
    let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
    let total_btc_balance: f64 = btc_wallets.iter().map(|w| w.balance).sum();
    
    if total_btc_balance > 0.0 {
        let btc_price_state = crate::wallet::evm::price_manager::get_cached_price_state("BTC");
        let btc_value = btc_price_state
            .price_usd
            .map(|price_usd| total_btc_balance * price_usd)
            .unwrap_or(0.0);

        result.push(UnifiedAsset {
            name: if btc_price_state.price_usd.is_none() {
                "Bitcoin (Price unavailable)".to_string()
            } else {
                "Bitcoin".to_string()
            },
            symbol: "BTC".to_string(),
            balance: total_btc_balance,
            usd_value: btc_value,
            chain: "Bitcoin".to_string(),
            is_testnet: false,
        });
    }

    // 2. EVM Assets
    let evm_wallets = db.get_evm_wallets().map_err(|e| e.to_string())?;
    for wallet in evm_wallets {
        let assets = db.get_evm_asset_balances(&wallet.id).map_err(|e| e.to_string())?;
        for (chain, symbol, chain_id, name, _, _, _, balance_float, _, usd_value, _) in assets {
            result.push(UnifiedAsset {
                name,
                symbol,
                balance: balance_float,
                usd_value: usd_value.unwrap_or(0.0),
                chain,
                is_testnet: chain_id == 11155111,
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{dashboard_freshness_from_row_with_failures, format_dashboard_stats};
    use crate::wallet::state::types::FreshnessStatus;
    use crate::wallet::types::ValuationStatus;

    #[test]
    fn dashboard_stats_hide_totals_when_btc_price_is_unavailable() {
        let stats = format_dashboard_stats(
            12_345.67,
            0.1289,
            321.0,
            2.6,
            crate::wallet::state::types::FreshnessMetadata {
                status: FreshnessStatus::Partial,
                updated_at: Some(1_713_499_200),
                failed_sources: vec!["price:btc_unavailable".to_string()],
            },
            ValuationStatus::Valued,
            0,
        );

        assert_eq!(stats.total_balance_usd, "$--");
        assert_eq!(stats.total_balance_btc, "≈ -- BTC");
        assert_eq!(stats.change_24h_amount, "--");
        assert_eq!(stats.change_24h_percentage, "--");
        assert!(matches!(stats.freshness.status, FreshnessStatus::Partial));
    }

    #[test]
    fn dashboard_stats_show_priced_subtotal_when_unpriced_assets_exist() {
        let stats = format_dashboard_stats(
            150.0,
            0.005,
            12.0,
            8.0,
            crate::wallet::state::types::FreshnessMetadata {
                status: FreshnessStatus::Fresh,
                updated_at: Some(1_713_499_200),
                failed_sources: Vec::new(),
            },
            ValuationStatus::Unpriced,
            2,
        );

        assert_eq!(stats.total_balance_usd, "$150.00");
        assert_eq!(stats.valuation_status, ValuationStatus::Unpriced);
        assert_eq!(stats.unpriced_asset_count, 2);
    }

    #[test]
    fn dashboard_row_with_failed_sources_degrades_to_partial() {
        let updated_at = chrono::Utc::now().to_rfc3339();
        let freshness = dashboard_freshness_from_row_with_failures(
            &updated_at,
            "[\"price:btc_unavailable\"]",
        );

        assert!(matches!(freshness.status, FreshnessStatus::Partial));
        assert_eq!(freshness.failed_sources, vec!["price:btc_unavailable".to_string()]);
    }
}
