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

#[tauri::command]
pub async fn evm_send_transaction(
    wallet_id: String,
    chain_id: u64,
    transaction: serde_json::Value,
) -> Result<String, String> {
    // Parse the transaction object
    let to = transaction.get("to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'to' field".to_string())?
        .to_string();
    
    let data = transaction.get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'data' field".to_string())?
        .to_string();
    
    let value = transaction.get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'value' field".to_string())?
        .to_string();
    
    let gas_limit = transaction.get("gasLimit")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'gasLimit' field".to_string())?
        .to_string();
    
    let gas_price = transaction.get("gasPrice")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'gasPrice' field".to_string())?
        .to_string();

    let request = crate::wallet::transaction_types::RawTransactionRequest {
        wallet_id,
        chain_id,
        to,
        data,
        value,
        gas_limit,
        gas_price,
    };

    let response = evm_transaction::send_raw_evm_transaction(request).await?;
    Ok(response.tx_hash)
}

#[tauri::command]
pub async fn evm_approve_token(
    wallet_id: String,
    chain_id: u64,
    token_address: String,
    spender_address: String,
    amount: String,
) -> Result<String, String> {
    let response = evm_transaction::approve_erc20_token(
        wallet_id,
        chain_id,
        token_address,
        spender_address,
        amount,
    ).await?;
    Ok(response.tx_hash)
}
