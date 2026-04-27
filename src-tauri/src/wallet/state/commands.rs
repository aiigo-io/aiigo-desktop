use crate::DB;

use super::{
    portfolio,
    types::{BalanceState, FreshnessMetadata, PortfolioState, PriceState},
};

const BALANCE_FRESH_WITHIN_SECS: i64 = 60;
const BALANCE_STALE_AFTER_SECS: i64 = 300;

fn bitcoin_balance_state_from_parts(balance: f64, freshness: FreshnessMetadata) -> BalanceState {
    BalanceState {
        raw_amount: balance.to_string(),
        display_amount: balance,
        chain_id: None,
        freshness,
    }
}

fn bitcoin_portfolio_state_from_items(
    items: &[(String, BalanceState)],
    price_state: &PriceState,
) -> PortfolioState {
    let portfolio_items: Vec<(String, BalanceState, PriceState)> = items
        .iter()
        .cloned()
        .map(|(source, balance_state)| (source, balance_state, price_state.clone()))
        .collect();

    portfolio::aggregate(&portfolio_items)
}

#[tauri::command]
pub fn state_get_bitcoin_wallet_balance_state(wallet_id: String) -> Result<BalanceState, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let wallet = db
        .get_bitcoin_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get bitcoin wallet: {}", e))?
        .ok_or_else(|| "Bitcoin wallet not found".to_string())?;
    let freshness = db
        .get_bitcoin_wallet_balance_freshness(
            &wallet_id,
            chrono::Utc::now().timestamp(),
            BALANCE_FRESH_WITHIN_SECS,
            BALANCE_STALE_AFTER_SECS,
        )
        .map_err(|e| format!("Failed to get bitcoin wallet freshness: {}", e))?
        .ok_or_else(|| "Bitcoin wallet freshness not found".to_string())?;

    Ok(bitcoin_balance_state_from_parts(wallet.balance, freshness))
}

#[tauri::command]
pub fn state_get_bitcoin_price_state() -> Result<PriceState, String> {
    Ok(crate::wallet::evm::price_manager::get_cached_price_state(
        "BTC",
    ))
}

#[tauri::command]
pub fn state_get_bitcoin_portfolio_state() -> Result<PortfolioState, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let wallets = db
        .get_bitcoin_wallets()
        .map_err(|e| format!("Failed to get bitcoin wallets: {}", e))?;
    let now = chrono::Utc::now().timestamp();

    let mut items = Vec::with_capacity(wallets.len());
    for wallet in wallets {
        let freshness = db
            .get_bitcoin_wallet_balance_freshness(
                &wallet.id,
                now,
                BALANCE_FRESH_WITHIN_SECS,
                BALANCE_STALE_AFTER_SECS,
            )
            .map_err(|e| format!("Failed to get bitcoin wallet freshness: {}", e))?
            .ok_or_else(|| format!("Bitcoin wallet freshness missing for {}", wallet.id))?;

        items.push((
            wallet.address,
            bitcoin_balance_state_from_parts(wallet.balance, freshness),
        ));
    }

    let price_state = crate::wallet::evm::price_manager::get_cached_price_state("BTC");
    Ok(bitcoin_portfolio_state_from_items(&items, &price_state))
}

#[cfg(test)]
mod tests {
    use super::{bitcoin_balance_state_from_parts, bitcoin_portfolio_state_from_items};
    use crate::wallet::state::{
        freshness, price,
        types::{FreshnessMetadata, FreshnessStatus},
    };
    use serde_json::Value;

    fn top_level_keys(value: &Value) -> Vec<String> {
        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    // M-CM-1
    #[test]
    fn state_command_balance_shape_uses_frozen_contract_keys() {
        let state = bitcoin_balance_state_from_parts(
            1.25,
            FreshnessMetadata {
                status: FreshnessStatus::Cached,
                updated_at: None,
                failed_sources: Vec::new(),
            },
        );

        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(
            top_level_keys(&json),
            vec!["chain_id", "display_amount", "freshness", "raw_amount"]
        );
    }

    #[test]
    fn state_command_price_shape_uses_frozen_contract_keys() {
        let state = price::unavailable();

        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(
            top_level_keys(&json),
            vec!["price_source", "price_updated_at", "price_usd", "status"]
        );
    }

    #[test]
    fn state_command_portfolio_shape_uses_frozen_contract_keys() {
        let state = bitcoin_portfolio_state_from_items(
            &[(
                "bc1test".to_string(),
                bitcoin_balance_state_from_parts(
                    2.0,
                    FreshnessMetadata {
                        status: freshness::classify_age(None, 100, 60, 300),
                        updated_at: None,
                        failed_sources: Vec::new(),
                    },
                ),
            )],
            &price::synthetic(95_000.0, "synthetic-stablecoin", 100),
        );

        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(
            top_level_keys(&json),
            vec!["freshness", "value_btc", "value_usd"]
        );
    }
}
