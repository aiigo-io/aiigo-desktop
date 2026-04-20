use crate::wallet::sync::engine;
use crate::wallet::sync::types::SyncReason;
use crate::wallet::types::WalletInfo;
use crate::DB;

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
    engine::sync_bitcoin_wallet_balance(&wallet_id, SyncReason::Manual)
        .await
        .map(|(wallet, _)| wallet)
}

#[tauri::command]
pub fn bitcoin_delete_wallet(wallet_id: String) -> Result<bool, String> {
    let db = DB.lock().unwrap();
    db.delete_bitcoin_wallet(&wallet_id)
        .map_err(|e| format!("Failed to delete wallet: {}", e))
}
