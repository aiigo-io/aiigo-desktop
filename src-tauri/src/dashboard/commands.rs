use crate::DB;
use crate::wallet::state::freshness::classify_age;
use crate::wallet::state::types::{FreshnessMetadata, FreshnessStatus, PriceStatus};
use crate::wallet::sync::engine;
use crate::wallet::sync::types::{SyncOutcome, SyncReason};
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

fn dashboard_freshness_from_sync(outcome: &SyncOutcome) -> FreshnessMetadata {
    FreshnessMetadata {
        status: if outcome.partial {
            FreshnessStatus::Partial
        } else {
            FreshnessStatus::Fresh
        },
        updated_at: outcome.updated_at,
        failed_sources: outcome.failed_sources.clone(),
    }
}

fn format_dashboard_stats(
    total_balance_usd: f64,
    total_balance_btc: f64,
    change_24h_amount: f64,
    change_24h_percentage: f64,
    freshness: FreshnessMetadata,
) -> DashboardStats {
    let price_unavailable = freshness
        .failed_sources
        .iter()
        .any(|source| source == "price:btc_unavailable");

    DashboardStats {
        total_balance_usd: if price_unavailable {
            "$--".to_string()
        } else {
            format!("${}", format_currency(total_balance_usd))
        },
        total_balance_btc: if price_unavailable {
            "≈ -- BTC".to_string()
        } else {
            format!("≈ {:.4} BTC", total_balance_btc)
        },
        change_24h_amount: if price_unavailable {
            "--".to_string()
        } else {
            format!("{}${}", if change_24h_amount >= 0.0 { "+" } else { "" }, format_currency(change_24h_amount))
        },
        change_24h_percentage: if price_unavailable {
            "--".to_string()
        } else {
            format!("{}{:.2}%", if change_24h_percentage >= 0.0 { "+" } else { "" }, change_24h_percentage)
        },
        freshness,
    }
}

#[tauri::command]
pub fn get_dashboard_stats() -> Result<DashboardStats, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let stats = db.get_dashboard_stats().map_err(|e| e.to_string())?;

    if let Some((usd, btc, change_amt, change_pct, updated_at, failed_sources_json)) = stats {
        let freshness = dashboard_freshness_from_row_with_failures(&updated_at, &failed_sources_json);
        Ok(format_dashboard_stats(usd, btc, change_amt, change_pct, freshness))
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
        ))
    }
}

#[tauri::command]
pub async fn refresh_dashboard_stats() -> Result<DashboardStats, String> {
    let (result, outcome) = engine::refresh_dashboard(SyncReason::Manual).await?;

    Ok(format_dashboard_stats(
        result.total_balance_usd,
        result.total_balance_btc,
        result.change_24h_amount,
        result.change_24h_percentage,
        dashboard_freshness_from_sync(&outcome),
    ))
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
    pub valuation_status: String,
}

#[tauri::command]
pub async fn get_asset_allocation() -> Result<Vec<AssetAllocation>, String> {
    let btc_price_state = crate::wallet::evm::price_manager::get_cached_price_state("BTC");

    let db = DB.lock().map_err(|e| e.to_string())?;
    
    let mut allocations: Vec<AssetAllocation> = Vec::new();
    let mut total_value: f64 = 0.0;
    
    // Get BTC wallets and their values
    let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
    let total_btc: f64 = btc_wallets.iter().map(|w| w.balance).sum();
    
    if total_btc > 0.0 {
        let btc_value_usd = match (btc_price_state.status, btc_price_state.price_usd) {
            (PriceStatus::Unavailable, _) | (_, None) => 0.0,
            (_, Some(price_usd)) => total_btc * price_usd,
        };
        allocations.push(AssetAllocation {
            name: if matches!(btc_price_state.status, PriceStatus::Unavailable) {
                "Bitcoin (Price unavailable)".to_string()
            } else {
                "Bitcoin".to_string()
            },
            symbol: "BTC".to_string(),
            percentage: 0.0, // Will be calculated after getting total
            value_usd: btc_value_usd,
            color: "bg-orange-500".to_string(),
            valuation_status: if matches!(btc_price_state.status, PriceStatus::Unavailable) {
                "unpriced".to_string()
            } else {
                "valued".to_string()
            },
        });
        if btc_value_usd > 0.0 {
            total_value += btc_value_usd;
        }
    }
    
    // Get EVM assets
    let evm_wallets = db.get_evm_wallets().map_err(|e| e.to_string())?;
    let mut evm_assets_map: std::collections::HashMap<String, (String, f64)> = std::collections::HashMap::new();
    
    for wallet in evm_wallets {
        let assets = db.get_evm_asset_balances(&wallet.id).map_err(|e| e.to_string())?;
        // Tuple: (chain, asset_symbol, chain_id, asset_name, contract_address, asset_decimals, balance, balance_float, usd_price, usd_value)
        for (_chain, symbol, chain_id, name, _contract_address, _decimals, _balance, _balance_float, _usd_price, usd_value) in assets {
            // Exclude Sepolia
            if chain_id != 11155111 {
                let entry = evm_assets_map.entry(symbol.clone()).or_insert((name, 0.0));
                entry.1 += usd_value;
                total_value += usd_value;
            }
        }
    }
    
    // Add EVM assets to allocations
    let colors = ["bg-blue-500", "bg-purple-500", "bg-cyan-500", "bg-green-500", "bg-pink-500", "bg-indigo-500"];
    let mut color_index = 0;
    
    for (symbol, (name, usd_value)) in evm_assets_map {
        if usd_value > 0.0 {
            allocations.push(AssetAllocation {
                name,
                symbol,
                percentage: 0.0,
                value_usd: usd_value,
                color: colors[color_index % colors.len()].to_string(),
                valuation_status: "valued".to_string(),
            });
            color_index += 1;
        }
    }
    
    // Calculate percentages
    if total_value > 0.0 {
        for allocation in &mut allocations {
            allocation.percentage = (allocation.value_usd / total_value) * 100.0;
        }
    }
    
    // Sort by value (descending)
    allocations.sort_by(|a, b| b.value_usd.partial_cmp(&a.value_usd).unwrap_or(std::cmp::Ordering::Equal));
    
    // Limit to top 5 and group the rest as "Other"
    if allocations.len() > 5 {
        let top_5: Vec<AssetAllocation> = allocations.drain(..5).collect();
        let other_value: f64 = allocations.iter().map(|a| a.value_usd).sum();
        let other_percentage = if total_value > 0.0 { (other_value / total_value) * 100.0 } else { 0.0 };
        
        allocations = top_5;
        if other_value > 0.0 {
            allocations.push(AssetAllocation {
                name: "Other".to_string(),
                symbol: "OTHER".to_string(),
                percentage: other_percentage,
                value_usd: other_value,
                color: "bg-slate-400".to_string(),
                valuation_status: "valued".to_string(),
            });
        }
    }
    
    Ok(allocations)
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
    let btc_price_state = crate::wallet::evm::price_manager::get_cached_price_state("BTC");
    let db = DB.lock().map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();

    // 1. BTC Wallets
    let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
    let total_btc_balance: f64 = btc_wallets.iter().map(|w| w.balance).sum();
    
    if total_btc_balance > 0.0 {
        let btc_value = match (btc_price_state.status, btc_price_state.price_usd) {
            (PriceStatus::Unavailable, _) | (_, None) => 0.0,
            (_, Some(price_usd)) => total_btc_balance * price_usd,
        };

        result.push(UnifiedAsset {
            name: if matches!(btc_price_state.status, PriceStatus::Unavailable) {
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
        for (chain, symbol, chain_id, name, _, _, _, balance_float, _, usd_value) in assets {
            result.push(UnifiedAsset {
                name,
                symbol,
                balance: balance_float,
                usd_value,
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
        );

        assert_eq!(stats.total_balance_usd, "$--");
        assert_eq!(stats.total_balance_btc, "≈ -- BTC");
        assert_eq!(stats.change_24h_amount, "--");
        assert_eq!(stats.change_24h_percentage, "--");
        assert!(matches!(stats.freshness.status, FreshnessStatus::Partial));
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
