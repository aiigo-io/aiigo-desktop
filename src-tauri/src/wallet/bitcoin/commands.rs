use crate::wallet::state::types::BalanceState;
use crate::wallet::sync::engine;
use crate::wallet::sync::types::SyncReason;
use crate::wallet::types::{
    BitcoinWalletBalanceResponse, FreshnessBackedBitcoinBalance, WalletInfo,
};
use crate::DB;

const BALANCE_FRESH_WITHIN_SECS: i64 = 60;
const BALANCE_STALE_AFTER_SECS: i64 = 300;

fn load_bitcoin_balance_state(wallet_id: &str) -> Result<BalanceState, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let wallet = db
        .get_bitcoin_wallet(wallet_id)
        .map_err(|e| format!("Failed to get bitcoin wallet: {}", e))?
        .ok_or_else(|| "Bitcoin wallet not found".to_string())?;
    let freshness = db
        .get_bitcoin_wallet_balance_freshness(
            wallet_id,
            chrono::Utc::now().timestamp(),
            BALANCE_FRESH_WITHIN_SECS,
            BALANCE_STALE_AFTER_SECS,
        )
        .map_err(|e| format!("Failed to get bitcoin wallet freshness: {}", e))?
        .ok_or_else(|| "Bitcoin wallet freshness not found".to_string())?;

    Ok(BalanceState {
        raw_amount: wallet.balance.to_string(),
        display_amount: wallet.balance,
        chain_id: None,
        freshness,
    })
}

fn to_bitcoin_balance_response(
    wallet: WalletInfo,
    balance_state: BalanceState,
) -> BitcoinWalletBalanceResponse {
    BitcoinWalletBalanceResponse {
        wallet,
        balance_state: FreshnessBackedBitcoinBalance {
            raw_amount: balance_state.raw_amount,
            display_amount: balance_state.display_amount,
            chain_id: balance_state.chain_id,
            freshness: balance_state.freshness,
        },
    }
}

#[tauri::command]
pub fn bitcoin_get_wallets() -> Result<Vec<WalletInfo>, String> {
    let db = DB.lock().unwrap();
    db.get_bitcoin_wallets()
        .map_err(|e| format!("Failed to get wallets: {}", e))
}

#[tauri::command]
pub fn bitcoin_get_wallet(wallet_id: String) -> Result<Option<WalletInfo>, String> {
    let db = DB.lock().unwrap();
    db.get_bitcoin_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))
}

#[tauri::command]
pub async fn bitcoin_get_wallet_with_balance(wallet_id: String) -> Result<WalletInfo, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    db.get_bitcoin_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))?
        .ok_or_else(|| "Wallet not found".to_string())
}

#[tauri::command]
pub fn query_bitcoin_wallet_balance(
    wallet_id: String,
) -> Result<BitcoinWalletBalanceResponse, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let wallet = db
        .get_bitcoin_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))?
        .ok_or_else(|| "Wallet not found".to_string())?;
    drop(db);

    let balance_state = load_bitcoin_balance_state(&wallet_id)?;
    Ok(to_bitcoin_balance_response(wallet, balance_state))
}

#[tauri::command]
pub async fn refresh_bitcoin_wallet_balance(
    wallet_id: String,
) -> Result<BitcoinWalletBalanceResponse, String> {
    let (wallet, _) = engine::sync_bitcoin_wallet_balance(&wallet_id, SyncReason::Manual).await?;
    let balance_state = load_bitcoin_balance_state(&wallet_id)?;
    Ok(to_bitcoin_balance_response(wallet, balance_state))
}

#[tauri::command]
pub fn bitcoin_delete_wallet(wallet_id: String) -> Result<bool, String> {
    let db = DB.lock().unwrap();
    db.delete_bitcoin_wallet(&wallet_id)
        .map_err(|e| format!("Failed to delete wallet: {}", e))
}
