use crate::wallet::bitcoin::transaction as btc_transaction;
use crate::wallet::evm::transaction as evm_transaction;
use crate::wallet::transaction_types::{
    BitcoinTransaction, EvmTransaction, SendBitcoinRequest, SendEvmRequest,
    SendTransactionResponse,
};
use crate::DB;

// Bitcoin Transaction Commands

#[tauri::command]
pub async fn send_bitcoin(request: SendBitcoinRequest) -> Result<SendTransactionResponse, String> {
    btc_transaction::send_bitcoin_transaction(request).await
}

#[tauri::command]
pub async fn get_bitcoin_transactions(wallet_id: String) -> Result<Vec<BitcoinTransaction>, String> {
    let db = DB.lock().unwrap();
    db.get_bitcoin_transactions(&wallet_id)
        .map_err(|e| format!("Failed to get transactions: {}", e))
}

#[tauri::command]
pub async fn get_all_bitcoin_transactions() -> Result<Vec<BitcoinTransaction>, String> {
    let db = DB.lock().unwrap();
    db.get_all_bitcoin_transactions()
        .map_err(|e| format!("Failed to get transactions: {}", e))
}

#[tauri::command]
pub async fn fetch_bitcoin_history(
    wallet_id: String,
    address: String,
) -> Result<Vec<BitcoinTransaction>, String> {
    btc_transaction::fetch_bitcoin_transaction_history(wallet_id, address).await
}

// EVM Transaction Commands

#[tauri::command]
pub async fn send_evm(request: SendEvmRequest) -> Result<SendTransactionResponse, String> {
    evm_transaction::send_evm_transaction(request).await
}

#[tauri::command]
pub async fn get_evm_transactions(wallet_id: String) -> Result<Vec<EvmTransaction>, String> {
    let db = DB.lock().unwrap();
    db.get_evm_transactions(&wallet_id)
        .map_err(|e| format!("Failed to get transactions: {}", e))
}

#[tauri::command]
pub async fn get_all_evm_transactions() -> Result<Vec<EvmTransaction>, String> {
    let db = DB.lock().unwrap();
    db.get_all_evm_transactions()
        .map_err(|e| format!("Failed to get transactions: {}", e))
}

#[tauri::command]
pub async fn fetch_evm_history(
    wallet_id: String,
    address: String,
    chain: String,
    chain_id: u64,
) -> Result<Vec<EvmTransaction>, String> {
    evm_transaction::fetch_evm_transaction_history(wallet_id, address, chain, chain_id).await
}
