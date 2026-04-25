use crate::wallet::bitcoin::transaction as bitcoin_transaction;
use crate::wallet::evm::transaction as evm_transaction;
use crate::wallet::security::commands::AppSecurity;
use crate::wallet::sync::engine;
use crate::wallet::sync::types::SyncReason;
use crate::wallet::transaction_types::{
    BitcoinFeeEstimationResponse, BitcoinTransaction, EvmTransaction, SendBitcoinRequest,
    SendEvmRequest, SendTransactionResponse,
};
use crate::DB;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SupportedEvmHistoryChain {
    pub chain: String,
    pub chain_id: u64,
    pub display_name: String,
}

// Bitcoin Transaction Commands

#[tauri::command]
pub async fn send_bitcoin(
    request: SendBitcoinRequest,
    state: tauri::State<'_, AppSecurity>,
) -> Result<SendTransactionResponse, String> {
    bitcoin_transaction::send_bitcoin_transaction(
        request,
        state.secret_backend(),
        state.keystore(),
        state.session_manager(),
    )
    .await
}

#[tauri::command]
pub async fn bitcoin_estimate_fees() -> Result<BitcoinFeeEstimationResponse, String> {
    bitcoin_transaction::estimate_bitcoin_fees().await
}

#[tauri::command]
pub async fn get_bitcoin_transactions(
    wallet_id: String,
) -> Result<Vec<BitcoinTransaction>, String> {
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
    engine::refresh_bitcoin_history(wallet_id, address, SyncReason::Manual)
        .await
        .map(|(transactions, _)| transactions)
}

// EVM Transaction Commands

#[tauri::command]
pub async fn send_evm(
    request: SendEvmRequest,
    state: tauri::State<'_, AppSecurity>,
) -> Result<SendTransactionResponse, String> {
    evm_transaction::send_evm_transaction(
        request,
        state.secret_backend(),
        state.keystore(),
        state.session_manager(),
    )
    .await
}

#[tauri::command]
pub async fn evm_estimate_gas(
    request: SendEvmRequest,
) -> Result<crate::wallet::transaction_types::EvmGasEstimationResponse, String> {
    evm_transaction::estimate_evm_gas(request).await
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
    let supported = crate::wallet::evm::config::get_history_sync_supported_chains()
        .into_iter()
        .any(|candidate| candidate.chain_id() == chain_id && candidate.name() == chain);

    if !supported {
        return Err(format!("unsupported_history_chain:{}:{}", chain, chain_id));
    }

    engine::refresh_evm_history(wallet_id, address, chain, chain_id, SyncReason::Manual)
        .await
        .map(|(transactions, _)| transactions)
}

#[tauri::command]
pub fn get_supported_evm_history_chains() -> Result<Vec<SupportedEvmHistoryChain>, String> {
    Ok(
        crate::wallet::evm::config::get_history_sync_supported_chains()
            .into_iter()
            .map(|chain| SupportedEvmHistoryChain {
                chain: chain.name().to_string(),
                chain_id: chain.chain_id(),
                display_name: chain.display_name().to_string(),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::get_supported_evm_history_chains;

    #[test]
    fn supported_history_chain_list_excludes_sepolia() {
        let supported = get_supported_evm_history_chains().unwrap();

        assert!(supported.iter().all(|chain| chain.chain_id != 11155111));
        assert!(supported.iter().any(|chain| chain.chain_id == 1));
    }
}

#[tauri::command]
pub async fn evm_send_transaction(
    wallet_id: String,
    chain_id: u64,
    transaction: serde_json::Value,
    state: tauri::State<'_, AppSecurity>,
) -> Result<String, String> {
    // Parse the transaction object
    let to = transaction
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'to' field".to_string())?
        .to_string();

    let data = transaction
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'data' field".to_string())?
        .to_string();

    let value = transaction
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'value' field".to_string())?
        .to_string();

    let gas_limit = transaction
        .get("gasLimit")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'gasLimit' field".to_string())?
        .to_string();

    let gas_price = transaction
        .get("gasPrice")
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

    let response = evm_transaction::send_raw_evm_transaction(
        request,
        state.secret_backend(),
        state.keystore(),
        state.session_manager(),
    )
    .await?;
    Ok(response.tx_hash)
}

#[tauri::command]
pub async fn evm_approve_token(
    wallet_id: String,
    chain_id: u64,
    token_address: String,
    spender_address: String,
    amount: String,
    state: tauri::State<'_, AppSecurity>,
) -> Result<String, String> {
    let response = evm_transaction::approve_erc20_token(
        wallet_id,
        chain_id,
        token_address,
        spender_address,
        amount,
        state.secret_backend(),
        state.keystore(),
        state.session_manager(),
    )
    .await?;
    Ok(response.tx_hash)
}
