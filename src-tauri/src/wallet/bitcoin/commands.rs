use crate::wallet::types::WalletInfo;
use crate::wallet::bitcoin::balance;
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
    // Get wallet info first, then release the database lock
    let wallet = {
        let db = DB.lock().unwrap();
        db.get_bitcoin_wallet(&wallet_id)
            .map_err(|e| format!("Failed to get wallet: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

    // Query balance from blockchain
    let balance = balance::query_btc_balance(&wallet.address).await?;

    // Update balance in database
    {
        let db = DB.lock().unwrap();
        db.update_bitcoin_wallet_balance(&wallet_id, balance)
            .map_err(|e| format!("Failed to update balance: {}", e))?;
    }

    // Return updated wallet info
    Ok(WalletInfo {
        id: wallet.id,
        label: wallet.label,
        wallet_type: wallet.wallet_type,
        address: wallet.address,
        balance,
        created_at: wallet.created_at,
        updated_at: wallet.updated_at,
    })
}

#[tauri::command]
pub fn bitcoin_delete_wallet(wallet_id: String) -> Result<bool, String> {
    let db = DB.lock().unwrap();
    db.delete_bitcoin_wallet(&wallet_id)
        .map_err(|e| format!("Failed to delete wallet: {}", e))
}
