use crate::db::Database;
use crate::wallet::evm::price_manager;
use crate::wallet::state::types::PriceStatus;
use crate::wallet::types::ValuationStatus;
use std::collections::HashMap;

const SEPOLIA_CHAIN_ID: u64 = 11155111;

#[derive(Debug, Clone)]
pub struct AllocationBucket {
    pub name: String,
    pub symbol: String,
    pub value_usd: f64,
    pub color: String,
    pub valuation_status: ValuationStatus,
}

#[derive(Debug, Clone)]
pub struct PortfolioValuationSnapshot {
    pub priced_total_usd: f64,
    pub unpriced_asset_count: usize,
    pub allocations: Vec<AllocationBucket>,
}

impl PortfolioValuationSnapshot {
    pub fn has_unpriced_assets(&self) -> bool {
        self.unpriced_asset_count > 0
    }

    pub fn valuation_status(&self) -> ValuationStatus {
        if self.has_unpriced_assets() {
            ValuationStatus::Unpriced
        } else {
            ValuationStatus::Valued
        }
    }
}

pub fn build_portfolio_valuation_snapshot(db: &Database) -> Result<PortfolioValuationSnapshot, String> {
    let btc_price_state = price_manager::get_cached_price_state("BTC");
    let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
    let evm_wallets = db.get_evm_wallets().map_err(|e| e.to_string())?;

    let mut priced_total_usd = 0.0;
    let mut unpriced_asset_count = 0;
    let mut priced_allocations = Vec::new();

    let total_btc: f64 = btc_wallets.iter().map(|wallet| wallet.balance).sum();
    if total_btc > 0.0 {
        match (btc_price_state.status, btc_price_state.price_usd) {
            (PriceStatus::Unavailable, _) | (_, None) => {
                unpriced_asset_count += 1;
            }
            (_, Some(price_usd)) => {
                let value_usd = total_btc * price_usd;
                priced_total_usd += value_usd;
                if value_usd > 0.0 {
                    priced_allocations.push(AllocationBucket {
                        name: "Bitcoin".to_string(),
                        symbol: "BTC".to_string(),
                        value_usd,
                        color: "bg-orange-500".to_string(),
                        valuation_status: ValuationStatus::Valued,
                    });
                }
            }
        }
    }

    let mut evm_assets_map: HashMap<String, (String, f64)> = HashMap::new();

    for wallet in evm_wallets {
        let assets = db.get_evm_asset_balances(&wallet.id).map_err(|e| e.to_string())?;
        for (_chain, symbol, chain_id, name, _contract_address, _decimals, _balance, balance_float, _usd_price, usd_value, valuation_status) in assets {
            if chain_id == SEPOLIA_CHAIN_ID || balance_float <= 0.0 {
                continue;
            }

            if valuation_status == "unpriced" {
                unpriced_asset_count += 1;
                continue;
            }

            let Some(usd_value) = usd_value else {
                unpriced_asset_count += 1;
                continue;
            };

            priced_total_usd += usd_value;
            let entry = evm_assets_map.entry(symbol.clone()).or_insert((name, 0.0));
            entry.1 += usd_value;
        }
    }

    let colors = [
        "bg-blue-500",
        "bg-purple-500",
        "bg-cyan-500",
        "bg-green-500",
        "bg-pink-500",
        "bg-indigo-500",
    ];

    let mut color_index = 0;
    for (symbol, (name, usd_value)) in evm_assets_map {
        if usd_value > 0.0 {
            priced_allocations.push(AllocationBucket {
                name,
                symbol,
                value_usd: usd_value,
                color: colors[color_index % colors.len()].to_string(),
                valuation_status: ValuationStatus::Valued,
            });
            color_index += 1;
        }
    }

    priced_allocations.sort_by(|a, b| {
        b.value_usd
            .partial_cmp(&a.value_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if priced_allocations.len() > 5 {
        let other_value = priced_allocations.iter().skip(5).map(|bucket| bucket.value_usd).sum();
        priced_allocations.truncate(5);
        if other_value > 0.0 {
            priced_allocations.push(AllocationBucket {
                name: "Other".to_string(),
                symbol: "OTHER".to_string(),
                value_usd: other_value,
                color: "bg-slate-400".to_string(),
                valuation_status: ValuationStatus::Valued,
            });
        }
    }

    if unpriced_asset_count > 0 {
        priced_allocations.push(AllocationBucket {
            name: format!("Unpriced Assets ({})", unpriced_asset_count),
            symbol: "UNPRICED".to_string(),
            value_usd: 0.0,
            color: "bg-amber-500".to_string(),
            valuation_status: ValuationStatus::Unpriced,
        });
    }

    Ok(PortfolioValuationSnapshot {
        priced_total_usd,
        unpriced_asset_count,
        allocations: priced_allocations,
    })
}

#[cfg(test)]
mod tests {
    use super::build_portfolio_valuation_snapshot;
    use crate::db::Database;
    use crate::wallet::security::secret_envelope::{StoredSecret, SECRET_FORMAT_PLAINTEXT_V0};

    #[test]
    fn allocation_snapshot_adds_unpriced_bucket_and_excludes_it_from_priced_total() {
        let db = Database::new(":memory:").unwrap();
        let wallet = db
            .insert_evm_wallet_with_secret(
                "Main".to_string(),
                "mnemonic".to_string(),
                "0xabc".to_string(),
                StoredSecret {
                    secret_data: "test-secret".to_string(),
                    secret_format: SECRET_FORMAT_PLAINTEXT_V0.to_string(),
                },
                "mnemonic".to_string(),
            )
            .unwrap();

        db.save_evm_asset_balance(
            wallet.id.clone(),
            "ethereum".to_string(),
            1,
            "ETH".to_string(),
            "Ethereum".to_string(),
            18,
            None,
            "1".to_string(),
            1.0,
            Some(100.0),
            Some(100.0),
            "valued".to_string(),
        )
        .unwrap();

        db.save_evm_asset_balance(
            wallet.id,
            "ethereum".to_string(),
            1,
            "ABC".to_string(),
            "Unpriced Token".to_string(),
            18,
            Some("0xdef".to_string()),
            "2".to_string(),
            2.0,
            None,
            None,
            "unpriced".to_string(),
        )
        .unwrap();

        let snapshot = build_portfolio_valuation_snapshot(&db).unwrap();

        assert_eq!(snapshot.priced_total_usd, 100.0);
        assert_eq!(snapshot.unpriced_asset_count, 1);
        assert_eq!(snapshot.allocations.len(), 2);
        assert_eq!(snapshot.allocations[1].symbol, "UNPRICED");
    }
}