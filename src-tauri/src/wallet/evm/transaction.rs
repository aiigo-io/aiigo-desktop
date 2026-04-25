use crate::wallet::evm::config::get_chain_by_id;
use crate::wallet::evm::private_key::{
    load_authorized_mnemonic, load_authorized_private_key, map_security_error,
};
use crate::wallet::security::backend::SecretBackend;
use crate::wallet::security::keystore::Keystore;
use crate::wallet::security::session::SessionManager;
use crate::wallet::security::types::{SecurityError, SignerOperation};
use crate::wallet::sync::types::EVM_MIN_BLOCK_DEPTH;
use crate::wallet::transaction_types::{
    EvmTransaction, SendEvmRequest, SendTransactionResponse, TransactionStatus, TransactionType,
};
use crate::wallet::types::WalletInfo;
use crate::DB;
use chrono::Utc;
use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use ethers::signers::coins_bip39::English;
use ethers::signers::MnemonicBuilder;
use ethers::types::{Address as EthAddress, TransactionReceipt, H256, U256};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

const ETHERSCAN_API_KEY: &str = "TKG5YYYPSX97W8HG7ZHA319UXKWMXFKKG6";

enum EvmSigningSecret {
    Mnemonic(String),
    PrivateKey(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct EtherscanTransaction {
    #[serde(rename = "blockNumber")]
    block_number: String,
    #[serde(rename = "timeStamp")]
    timestamp: String,
    hash: String,
    from: String,
    to: String,
    value: String,
    #[serde(default)]
    gas: String,
    #[serde(rename = "gasPrice", default)]
    gas_price: String,
    #[serde(rename = "gasUsed", default)]
    gas_used: String,
    #[serde(rename = "isError", default)]
    is_error: String,
    #[serde(rename = "contractAddress", default)]
    contract_address: String,
    #[serde(rename = "tokenName")]
    token_name: Option<String>,
    #[serde(rename = "tokenSymbol")]
    token_symbol: Option<String>,
    #[serde(rename = "tokenDecimal")]
    token_decimal: Option<String>,
    #[serde(default)]
    input: String,
    #[serde(rename = "methodId", default)]
    method_id: String,
}

impl Default for EtherscanTransaction {
    fn default() -> Self {
        Self {
            block_number: String::new(),
            timestamp: String::new(),
            hash: String::new(),
            from: String::new(),
            to: String::new(),
            value: String::new(),
            gas: String::new(),
            gas_price: String::new(),
            gas_used: String::new(),
            is_error: "0".to_string(),
            contract_address: String::new(),
            token_name: None,
            token_symbol: None,
            token_decimal: None,
            input: String::new(),
            method_id: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct EtherscanResponse {
    status: String,
    message: String,
    result: serde_json::Value,
}

fn get_etherscan_api_url(chain_id: u64) -> Option<String> {
    match chain_id {
        1 => Some("https://api.etherscan.io/v2/api".to_string()),
        56 => Some("https://api.bscscan.com/v2/api".to_string()),
        137 => Some("https://api.polygonscan.com/v2/api".to_string()),
        42161 => Some("https://api.arbiscan.io/v2/api".to_string()),
        10 => Some("https://api-optimistic.etherscan.io/v2/api".to_string()),
        _ => None,
    }
}

fn estimated_fee_snapshot(gas_limit: U256, gas_price: U256) -> (String, String, f64) {
    let fee_wei = gas_limit.saturating_mul(gas_price);
    let fee = fee_wei.as_u128() as f64 / 1e18;

    (gas_limit.to_string(), gas_price.to_string(), fee)
}

/// Fetch EVM transaction history from the blockchain using Etherscan API
pub async fn fetch_evm_transaction_history(
    wallet_id: String,
    address: String,
    chain: String,
    chain_id: u64,
) -> Result<Vec<EvmTransaction>, String> {
    let api_url = get_etherscan_api_url(chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported for Etherscan API", chain_id))?;

    let chain_config =
        get_chain_by_id(chain_id).ok_or_else(|| format!("Chain ID {} not supported", chain_id))?;

    let mut all_transactions = Vec::new();

    // Fetch normal transactions
    let normal_txs = fetch_normal_transactions(&api_url, &address, chain_id).await?;
    all_transactions.extend(normal_txs);

    // Fetch ERC20 token transfers
    let token_txs = fetch_token_transactions(&api_url, &address, chain_id).await?;
    all_transactions.extend(token_txs);

    // Convert to EvmTransaction and save to database
    let mut result = Vec::new();
    let native_symbol = chain_config
        .assets()
        .first()
        .map(|a| a.symbol.clone())
        .unwrap_or_else(|| "ETH".to_string());

    for tx in all_transactions {
        let is_from_me = tx.from.to_lowercase() == address.to_lowercase();

        let tx_type = if is_from_me {
            if tx.method_id == "0x095ea7b3" || tx.input.starts_with("0x095ea7b3") {
                TransactionType::Approve
            } else if !tx.input.is_empty() && tx.input != "0x" {
                TransactionType::Contract
            } else {
                TransactionType::Send
            }
        } else {
            TransactionType::Receive
        };

        let block_number = tx.block_number.parse::<u64>().ok();
        let status = TransactionStatus::from_evm_receipt(
            Some(tx.is_error == "0"),
            Some(EVM_MIN_BLOCK_DEPTH),
            EVM_MIN_BLOCK_DEPTH,
        );

        let timestamp_secs = tx
            .timestamp
            .parse::<i64>()
            .unwrap_or_else(|_| Utc::now().timestamp());
        let timestamp = chrono::DateTime::from_timestamp(timestamp_secs, 0)
            .unwrap_or_else(|| Utc::now())
            .to_rfc3339();

        // Determine if it's a token transfer or native transfer
        let (asset_symbol, asset_name, contract_address, decimals) =
            if !tx.contract_address.is_empty() {
                // ERC20 token transfer
                let symbol = tx
                    .token_symbol
                    .clone()
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                let name = tx
                    .token_name
                    .clone()
                    .unwrap_or_else(|| "Unknown Token".to_string());
                let decimals = tx
                    .token_decimal
                    .as_ref()
                    .and_then(|d| d.parse::<u8>().ok())
                    .unwrap_or(18);
                (symbol, name, Some(tx.contract_address.clone()), decimals)
            } else {
                // Native token transfer
                (native_symbol.clone(), native_symbol.clone(), None, 18)
            };

        // Calculate amount
        let value_u256 = U256::from_dec_str(&tx.value).unwrap_or_default();
        let amount_float = if decimals > 0 {
            let divisor = 10_u128.pow(decimals as u32);
            value_u256.as_u128() as f64 / divisor as f64
        } else {
            value_u256.as_u128() as f64
        };

        // Calculate fee (only for transactions sent by this wallet)
        let fee = if tx_type == TransactionType::Send {
            let gas_used_u256 = U256::from_dec_str(&tx.gas_used).unwrap_or_default();
            let gas_price_u256 = U256::from_dec_str(&tx.gas_price).unwrap_or_default();
            let fee_wei = gas_used_u256 * gas_price_u256;
            fee_wei.as_u128() as f64 / 1e18
        } else {
            0.0
        };

        let evm_tx = EvmTransaction {
            id: Uuid::new_v4().to_string(),
            wallet_id: wallet_id.clone(),
            tx_hash: tx.hash.clone(),
            tx_type,
            from_address: tx.from.clone(),
            to_address: tx.to.clone(),
            amount: tx.value.clone(),
            amount_float,
            asset_symbol,
            asset_name,
            contract_address,
            chain: chain.clone(),
            chain_id,
            gas_used: tx.gas_used.clone(),
            gas_price: tx.gas_price.clone(),
            fee,
            status,
            block_number,
            timestamp: timestamp.clone(),
            created_at: timestamp,
        };

        // Save to database
        {
            let db = DB.lock().unwrap();
            db.add_evm_transaction(&evm_tx)
                .map_err(|e| format!("Failed to save transaction: {}", e))?;
        }

        result.push(evm_tx);
    }

    Ok(result)
}

async fn fetch_normal_transactions(
    api_url: &str,
    address: &str,
    chain_id: u64,
) -> Result<Vec<EtherscanTransaction>, String> {
    let url = format!(
        "{}?chainid={}&module=account&action=txlist&address={}&startblock=0&endblock=99999999&sort=desc&apikey={}",
        api_url, chain_id, address, ETHERSCAN_API_KEY
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch transactions: {}", e))?;

    let etherscan_response: EtherscanResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if etherscan_response.status != "1" {
        if etherscan_response.message.contains("No transactions found") {
            return Ok(Vec::new());
        }
        return Err(format!(
            "Etherscan API error: {}",
            etherscan_response.message
        ));
    }

    let transactions: Vec<EtherscanTransaction> =
        serde_json::from_value(etherscan_response.result.clone())
            .map_err(|e| format!("Failed to parse normal transactions: {}", e))?;

    Ok(transactions)
}

async fn fetch_token_transactions(
    api_url: &str,
    address: &str,
    chain_id: u64,
) -> Result<Vec<EtherscanTransaction>, String> {
    let url = format!(
        "{}?chainid={}&module=account&action=tokentx&address={}&startblock=0&endblock=99999999&sort=desc&apikey={}",
        api_url, chain_id, address, ETHERSCAN_API_KEY
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch token transactions: {}", e))?;

    let etherscan_response: EtherscanResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if etherscan_response.status != "1" {
        if etherscan_response.message.contains("No transactions found") {
            return Ok(Vec::new());
        }
        return Err(format!(
            "Etherscan API error: {}",
            etherscan_response.message
        ));
    }

    let transactions: Vec<EtherscanTransaction> =
        serde_json::from_value(etherscan_response.result.clone())
            .map_err(|e| format!("Failed to parse token transactions: {}", e))?;

    Ok(transactions)
}

/// Estimate gas for EVM transaction
pub async fn estimate_evm_gas(
    request: SendEvmRequest,
) -> Result<crate::wallet::transaction_types::EvmGasEstimationResponse, String> {
    // Get wallet info
    let wallet_info = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet(&request.wallet_id)
            .map_err(|e| format!("Failed to get wallet info: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

    // Get chain config
    let chain_config = get_chain_by_id(request.chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported", request.chain_id))?;

    // Create provider
    let provider = Provider::<Http>::try_from(chain_config.rpc_url())
        .map_err(|e| format!("Failed to create provider: {}", e))?;

    // Parse recipient address
    let to_address = EthAddress::from_str(&request.to_address)
        .map_err(|e| format!("Invalid recipient address: {}", e))?;

    // Parse amount
    let amount_wei =
        U256::from_dec_str(&request.amount).map_err(|e| format!("Invalid amount: {}", e))?;

    let from_address = EthAddress::from_str(&wallet_info.address)
        .map_err(|e| format!("Invalid wallet address: {}", e))?;

    let gas_limit: U256;
    let gas_price: U256;

    // Check if it's a native token or ERC20 transfer
    if request.contract_address.is_none() {
        // Native token transfer
        let tx = TransactionRequest::new()
            .from(from_address)
            .to(to_address)
            .value(amount_wei);

        gas_limit = provider
            .estimate_gas(&tx.into(), None)
            .await
            .map_err(|e| format!("Failed to estimate gas: {}", e))?;
    } else {
        // ERC20 token transfer
        let contract_address_str = request.contract_address.as_ref().unwrap();
        let contract_address = EthAddress::from_str(contract_address_str)
            .map_err(|e| format!("Invalid contract address: {}", e))?;

        // ERC20 ABI for transfer function
        let abi = ethers::abi::parse_abi(&[
            "function transfer(address to, uint256 amount) returns (bool)",
        ])
        .map_err(|e| format!("Failed to parse ABI: {}", e))?;

        let data = abi
            .function("transfer")
            .unwrap()
            .encode_input(&[
                ethers::abi::Token::Address(to_address),
                ethers::abi::Token::Uint(amount_wei),
            ])
            .map_err(|e| format!("Failed to encode transfer data: {}", e))?;

        let tx = TransactionRequest::new()
            .from(from_address)
            .to(contract_address)
            .data(data);

        gas_limit = provider
            .estimate_gas(&tx.into(), None)
            .await
            .map_err(|e| format!("Failed to estimate gas: {}", e))?;
    }

    gas_price = provider
        .get_gas_price()
        .await
        .map_err(|e| format!("Failed to get gas price: {}", e))?;

    Ok(crate::wallet::transaction_types::EvmGasEstimationResponse {
        gas_limit: gas_limit.as_u64(),
        gas_price: gas_price.to_string(),
    })
}

/// Send EVM transaction
pub async fn send_evm_transaction(
    request: SendEvmRequest,
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
) -> Result<SendTransactionResponse, String> {
    // Get wallet info
    let wallet_info = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet(&request.wallet_id)
            .map_err(|e| format!("Failed to get wallet info: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

    let signing_secret = load_signing_secret(
        &wallet_info,
        secret_backend,
        keystore,
        session_manager,
        SignerOperation::Send,
    )
    .map_err(map_security_error)?
    .ok_or_else(|| "Wallet secret not found".to_string())?;

    // Get chain config
    let chain_config = get_chain_by_id(request.chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported", request.chain_id))?;

    // Create provider
    let provider = Provider::<Http>::try_from(chain_config.rpc_url())
        .map_err(|e| format!("Failed to create provider: {}", e))?;
    let provider = Arc::new(provider);

    // Create wallet from secret
    let wallet = wallet_from_signing_secret(signing_secret, request.chain_id)?;

    let client = SignerMiddleware::new(provider.clone(), wallet);

    // Parse recipient address
    let to_address = EthAddress::from_str(&request.to_address)
        .map_err(|e| format!("Invalid recipient address: {}", e))?;

    // Parse amount
    let amount_wei =
        U256::from_dec_str(&request.amount).map_err(|e| format!("Invalid amount: {}", e))?;
    let from_address = EthAddress::from_str(&wallet_info.address)
        .map_err(|e| format!("Invalid wallet address: {}", e))?;

    let tx_hash: H256;
    let gas_used_str: String;
    let gas_price_str: String;
    let fee: f64;
    // Check if it's a native token or ERC20 transfer
    if request.contract_address.is_none() {
        // Native token transfer (ETH, etc.)
        let gas_limit = if let Some(gas_limit) = request.gas_limit {
            U256::from(gas_limit)
        } else {
            provider
                .estimate_gas(
                    &TransactionRequest::new()
                        .from(from_address)
                        .to(to_address)
                        .value(amount_wei)
                        .into(),
                    None,
                )
                .await
                .map_err(|e| format!("Failed to estimate gas: {}", e))?
        };
        let gas_price = if let Some(gas_price) = &request.gas_price {
            U256::from_dec_str(gas_price).map_err(|e| format!("Invalid gas price: {}", e))?
        } else {
            provider
                .get_gas_price()
                .await
                .map_err(|e| format!("Failed to get gas price: {}", e))?
        };

        let tx = TransactionRequest::new()
            .to(to_address)
            .value(amount_wei)
            .gas(gas_limit)
            .gas_price(gas_price);

        // Send transaction
        let pending_tx = client
            .send_transaction(tx, None)
            .await
            .map_err(|e| format!("Failed to send transaction: {}", e))?;

        tx_hash = *pending_tx;
        (gas_used_str, gas_price_str, fee) = estimated_fee_snapshot(gas_limit, gas_price);
    } else {
        // ERC20 token transfer
        let contract_address_str = request.contract_address.as_ref().unwrap();
        let contract_address = EthAddress::from_str(contract_address_str)
            .map_err(|e| format!("Invalid contract address: {}", e))?;

        // ERC20 ABI for transfer function
        let abi = ethers::abi::parse_abi(&[
            "function transfer(address to, uint256 amount) returns (bool)",
        ])
        .map_err(|e| format!("Failed to parse ABI: {}", e))?;

        let transfer_data = abi
            .function("transfer")
            .unwrap()
            .encode_input(&[
                ethers::abi::Token::Address(to_address),
                ethers::abi::Token::Uint(amount_wei),
            ])
            .map_err(|e| format!("Failed to encode transfer data: {}", e))?;

        let gas_limit = if let Some(gas_limit) = request.gas_limit {
            U256::from(gas_limit)
        } else {
            provider
                .estimate_gas(
                    &TransactionRequest::new()
                        .from(from_address)
                        .to(contract_address)
                        .data(transfer_data)
                        .into(),
                    None,
                )
                .await
                .map_err(|e| format!("Failed to estimate gas: {}", e))?
        };
        let gas_price = if let Some(gas_price) = &request.gas_price {
            U256::from_dec_str(gas_price).map_err(|e| format!("Invalid gas price: {}", e))?
        } else {
            provider
                .get_gas_price()
                .await
                .map_err(|e| format!("Failed to get gas price: {}", e))?
        };

        let contract = Contract::new(contract_address, abi, Arc::new(client.clone()));

        // Call transfer function
        let call = contract
            .method::<_, bool>("transfer", (to_address, amount_wei))
            .map_err(|e| format!("Failed to create contract call: {}", e))?
            .gas(gas_limit)
            .gas_price(gas_price);

        let pending_tx = call
            .send()
            .await
            .map_err(|e| format!("Failed to send transaction: {}", e))?;

        tx_hash = *pending_tx;
        (gas_used_str, gas_price_str, fee) = estimated_fee_snapshot(gas_limit, gas_price);
    }

    // Find decimals for the asset
    let decimals = if let Some(addr) = &request.contract_address {
        chain_config
            .assets()
            .iter()
            .find(|a| {
                a.contract_address.as_ref().map(|ca| ca.to_lowercase()) == Some(addr.to_lowercase())
            })
            .map(|a| a.decimals)
            .unwrap_or(18)
    } else {
        18
    };

    // Parse amount as float for display
    let amount_float = U256::from_dec_str(&request.amount)
        .map(|v| v.as_u128() as f64 / 10f64.powi(decimals as i32))
        .unwrap_or(0.0);

    // Get asset name from config if possible
    let asset_name = if let Some(addr) = &request.contract_address {
        chain_config
            .assets()
            .iter()
            .find(|a| {
                a.contract_address.as_ref().map(|ca| ca.to_lowercase()) == Some(addr.to_lowercase())
            })
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "".to_string())
    } else {
        request.asset_symbol.clone() // For native token, symbol and name are often same
    };

    // Save transaction to database
    let now = Utc::now().to_rfc3339();
    let tx_record = EvmTransaction {
        id: Uuid::new_v4().to_string(),
        wallet_id: request.wallet_id.clone(),
        tx_hash: format!("{:?}", tx_hash),
        tx_type: TransactionType::Send,
        from_address: wallet_info.address.clone(),
        to_address: request.to_address,
        amount: request.amount,
        amount_float,
        asset_symbol: request.asset_symbol,
        asset_name,
        contract_address: request.contract_address,
        chain: request.chain,
        chain_id: request.chain_id,
        gas_used: gas_used_str,
        gas_price: gas_price_str,
        fee,
        status: TransactionStatus::after_broadcast(),
        block_number: None,
        timestamp: now.clone(),
        created_at: now,
    };

    {
        let db = DB.lock().unwrap();
        db.add_evm_transaction(&tx_record)
            .map_err(|e| format!("Failed to save transaction: {}", e))?;
    }

    Ok(SendTransactionResponse {
        tx_hash: format!("{:?}", tx_hash),
        message: "Transaction sent successfully".to_string(),
    })
}

#[cfg(test)]
mod estimated_fee_tests {
    use super::estimated_fee_snapshot;
    use crate::wallet::transaction_types::TransactionStatus;
    use ethers::types::U256;

    #[test]
    fn estimated_fee_snapshot_persists_known_non_zero_values() {
        let (gas_used, gas_price, fee) =
            estimated_fee_snapshot(U256::from(65_000_u64), U256::from(1_500_000_000_u64));

        assert_eq!(gas_used, "65000");
        assert_eq!(gas_price, "1500000000");
        assert!(fee > 0.0);
    }

    #[test]
    fn broadcast_status_stays_broadcasted_after_send_snapshot() {
        assert_eq!(TransactionStatus::after_broadcast().as_str(), "broadcasted");
        assert_ne!(
            TransactionStatus::after_broadcast(),
            TransactionStatus::Confirmed
        );
    }
}

/// Helper to get transaction receipt
#[allow(dead_code)]
pub async fn get_transaction_receipt(
    tx_hash: String,
    chain_id: u64,
) -> Result<Option<TransactionReceipt>, String> {
    // Get chain config
    let chain_config =
        get_chain_by_id(chain_id).ok_or_else(|| format!("Chain ID {} not supported", chain_id))?;

    // Create provider
    let provider = Provider::<Http>::try_from(chain_config.rpc_url())
        .map_err(|e| format!("Failed to create provider: {}", e))?;

    // Parse transaction hash
    let hash = H256::from_str(&tx_hash).map_err(|e| format!("Invalid transaction hash: {}", e))?;

    // Get receipt
    let receipt = provider
        .get_transaction_receipt(hash)
        .await
        .map_err(|e| format!("Failed to get receipt: {}", e))?;

    Ok(receipt)
}

/// Send a raw EVM transaction (for OpenOcean swaps and other contract interactions)
pub async fn send_raw_evm_transaction(
    request: crate::wallet::transaction_types::RawTransactionRequest,
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
) -> Result<SendTransactionResponse, String> {
    // Get wallet info
    let wallet_info = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet(&request.wallet_id)
            .map_err(|e| format!("Failed to get wallet info: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

    let signing_secret = load_signing_secret(
        &wallet_info,
        secret_backend,
        keystore,
        session_manager,
        SignerOperation::Send,
    )
    .map_err(map_security_error)?
    .ok_or_else(|| "Wallet secret not found".to_string())?;

    // Get chain config
    let chain_config = get_chain_by_id(request.chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported", request.chain_id))?;

    // Create provider
    let provider = Provider::<Http>::try_from(chain_config.rpc_url())
        .map_err(|e| format!("Failed to create provider: {}", e))?;
    let provider = Arc::new(provider);

    // Create wallet from secret
    let wallet = wallet_from_signing_secret(signing_secret, request.chain_id)?;

    let client = SignerMiddleware::new(provider.clone(), wallet);

    // Parse recipient address
    let to_address = EthAddress::from_str(&request.to)
        .map_err(|e| format!("Invalid recipient address: {}", e))?;

    // Parse value (amount of native token to send)
    let value = U256::from_dec_str(&request.value).map_err(|e| format!("Invalid value: {}", e))?;

    // Parse gas limit
    let gas_limit =
        U256::from_dec_str(&request.gas_limit).map_err(|e| format!("Invalid gas limit: {}", e))?;

    // Parse gas price (in wei)
    let gas_price =
        U256::from_dec_str(&request.gas_price).map_err(|e| format!("Invalid gas price: {}", e))?;

    // Parse data (hex string)
    let data = if request.data.starts_with("0x") {
        hex::decode(&request.data[2..]).map_err(|e| format!("Invalid data hex: {}", e))?
    } else {
        hex::decode(&request.data).map_err(|e| format!("Invalid data hex: {}", e))?
    };

    // Build transaction
    let tx = TransactionRequest::new()
        .to(to_address)
        .value(value)
        .data(data)
        .gas(gas_limit)
        .gas_price(gas_price);

    // Send transaction
    let pending_tx = client
        .send_transaction(tx, None)
        .await
        .map_err(|e| format!("Failed to send transaction: {}", e))?;

    let tx_hash = *pending_tx;
    let tx_hash_str = format!("{:?}", tx_hash);

    // Don't wait for confirmation, return immediately
    // The frontend can track the transaction status separately

    Ok(SendTransactionResponse {
        tx_hash: tx_hash_str,
        message: "Transaction sent successfully".to_string(),
    })
}

/// Approve ERC20 token spending (for swaps and other contract interactions)
pub async fn approve_erc20_token(
    wallet_id: String,
    chain_id: u64,
    token_address: String,
    spender_address: String,
    amount: String,
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
) -> Result<SendTransactionResponse, String> {
    let wallet_info = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet(&wallet_id)
            .map_err(|e| format!("Failed to get wallet info: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

    approve_erc20_token_with_wallet(
        wallet_info,
        chain_id,
        token_address,
        spender_address,
        amount,
        secret_backend,
        keystore,
        session_manager,
    )
    .await
}

async fn approve_erc20_token_with_wallet(
    wallet_info: WalletInfo,
    chain_id: u64,
    token_address: String,
    spender_address: String,
    amount: String,
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
) -> Result<SendTransactionResponse, String> {
    let signing_secret = load_signing_secret(
        &wallet_info,
        secret_backend,
        keystore,
        session_manager,
        SignerOperation::Approve,
    )
    .map_err(map_security_error)?
    .ok_or_else(|| "Wallet secret not found".to_string())?;

    // Get chain config
    let chain_config =
        get_chain_by_id(chain_id).ok_or_else(|| format!("Chain ID {} not supported", chain_id))?;

    // Parse token contract address
    let contract_address = EthAddress::from_str(&token_address)
        .map_err(|e| format!("Invalid token address: {}", e))?;

    // Parse amount to approve (handle both hex and decimal formats)
    let approve_amount = if amount.starts_with("0x") || amount.starts_with("0X") {
        // Hex format (e.g., "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
        U256::from_str_radix(&amount[2..], 16).map_err(|e| format!("Invalid hex amount: {}", e))?
    } else {
        // Decimal format
        U256::from_dec_str(&amount).map_err(|e| format!("Invalid decimal amount: {}", e))?
    };

    // ERC20 ABI for approve function
    let abi = ethers::abi::parse_abi(&[
        "function approve(address spender, uint256 amount) returns (bool)",
    ])
    .map_err(|e| format!("Failed to parse ABI: {}", e))?;

    // Parse spender address from parameter (OpenOcean router address)
    let spender = EthAddress::from_str(&spender_address)
        .map_err(|e| format!("Invalid spender address: {}", e))?;

    // Create provider only after request validation succeeds.
    let provider = Provider::<Http>::try_from(chain_config.rpc_url())
        .map_err(|e| format!("Failed to create provider: {}", e))?;
    let provider = Arc::new(provider);

    // Create wallet from secret
    let wallet = wallet_from_signing_secret(signing_secret, chain_id)?;
    let client = SignerMiddleware::new(provider.clone(), wallet);

    // Call approve function
    let contract = Contract::new(contract_address, abi, Arc::new(client.clone()));
    let call = contract
        .method::<_, bool>("approve", (spender, approve_amount))
        .map_err(|e| format!("Failed to create approve call: {}", e))?;

    let pending_tx = call
        .send()
        .await
        .map_err(|e| format!("Failed to send approval transaction: {}", e))?;

    let tx_hash = *pending_tx;
    let tx_hash_str = format!("{:?}", tx_hash);

    // Approval refresh stays inline until the shared sync engine consumes
    // approval polling as a first-class flow.
    let _receipt = pending_tx
        .await
        .map_err(|e| format!("Failed to get receipt: {}", e))?
        .ok_or_else(|| "Approval transaction failed".to_string())?;

    Ok(SendTransactionResponse {
        tx_hash: tx_hash_str,
        message: "Token approved successfully".to_string(),
    })
}

fn load_signing_secret(
    wallet_info: &WalletInfo,
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
    operation: SignerOperation,
) -> Result<Option<EvmSigningSecret>, SecurityError> {
    match wallet_info.wallet_type.as_str() {
        "mnemonic" => Ok(load_authorized_mnemonic(
            &wallet_info.address,
            secret_backend,
            keystore,
            session_manager,
            operation,
        )?
        .map(EvmSigningSecret::Mnemonic)),
        "private-key" | "private_key" => Ok(load_authorized_private_key(
            &wallet_info.address,
            secret_backend,
            keystore,
            session_manager,
            operation,
        )?
        .map(EvmSigningSecret::PrivateKey)),
        _ => Ok(None),
    }
}

fn wallet_from_signing_secret(
    signing_secret: EvmSigningSecret,
    chain_id: u64,
) -> Result<LocalWallet, String> {
    match signing_secret {
        EvmSigningSecret::Mnemonic(secret_data) => MnemonicBuilder::<English>::default()
            .phrase(secret_data.as_str())
            .derivation_path("m/44'/60'/0'/0/0")
            .map_err(|e| format!("Failed to set derivation path: {}", e))?
            .build()
            .map_err(|e| format!("Failed to build wallet: {}", e))
            .map(|wallet| wallet.with_chain_id(chain_id)),
        EvmSigningSecret::PrivateKey(secret_data) => {
            let trimmed = secret_data.trim();
            let pk = if trimmed.starts_with("0x") {
                trimmed.to_string()
            } else {
                format!("0x{}", trimmed)
            };
            pk.parse::<LocalWallet>()
                .map_err(|e| format!("Failed to parse private key: {}", e))
                .map(|wallet| wallet.with_chain_id(chain_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{approve_erc20_token, load_signing_secret, EvmSigningSecret};
    use crate::db::Database;
    use crate::wallet::security::backend::{SecretBackend, SecretBackendAdapter};
    use crate::wallet::security::keystore::{Keystore, SqliteKeystore};
    use crate::wallet::security::secret_envelope::{
        decrypt_secret, encrypt_secret, SecretEnvelopeError, StoredSecret,
        SECRET_FORMAT_PLAINTEXT_V0,
    };
    use crate::wallet::security::session::SessionManager;
    use crate::wallet::security::types::{SecurityError, SignerOperation};
    use crate::wallet::types::WalletInfo;
    use crate::DB;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    struct PanicKeystore;

    struct DatabaseBackedKeystore {
        db: Database,
    }

    impl Keystore for PanicKeystore {
        fn load_mnemonic(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            panic!("keystore should not be called while session is locked");
        }

        fn load_private_key(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            panic!("keystore should not be called while session is locked");
        }
    }

    impl DatabaseBackedKeystore {
        fn load_secret(
            &self,
            address: &str,
        ) -> Result<Option<(String, String, String)>, SecurityError> {
            if let Some(wallet_id) = self
                .db
                .get_evm_wallets()
                .map_err(|_| SecurityError::OperationNotAllowed)?
                .into_iter()
                .find(|wallet| wallet.address == address)
                .map(|wallet| wallet.id)
            {
                return self
                    .db
                    .load_evm_wallet_secret(&wallet_id)
                    .map_err(|_| SecurityError::OperationNotAllowed);
            }

            Err(SecurityError::UnknownWallet)
        }
    }

    impl Keystore for DatabaseBackedKeystore {
        fn load_mnemonic(&self, address: &str) -> Result<Option<String>, SecurityError> {
            Ok(self
                .load_secret(address)?
                .and_then(|(secret_data, secret_type, secret_format)| {
                    (secret_type == "mnemonic").then_some((secret_data, secret_format))
                })
                .map(|(secret_data, secret_format)| {
                    decrypt_secret(&secret_data, &secret_format)
                        .map_err(|_| SecurityError::OperationNotAllowed)
                })
                .transpose()?)
        }

        fn load_private_key(&self, address: &str) -> Result<Option<String>, SecurityError> {
            Ok(self
                .load_secret(address)?
                .and_then(|(secret_data, secret_type, secret_format)| {
                    (secret_type == "private-key").then_some((secret_data, secret_format))
                })
                .map(|(secret_data, secret_format)| {
                    decrypt_secret(&secret_data, &secret_format)
                        .map_err(|_| SecurityError::OperationNotAllowed)
                })
                .transpose()?)
        }
    }

    struct TestSecretBackendAdapter;

    impl SecretBackendAdapter for TestSecretBackendAdapter {
        fn probe(&self) -> Result<(), SecretEnvelopeError> {
            Ok(())
        }

        fn encrypt(&self, plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
            encrypt_secret(plaintext)
        }

        fn decrypt(
            &self,
            secret_data: &str,
            secret_format: &str,
        ) -> Result<String, SecretEnvelopeError> {
            decrypt_secret(secret_data, secret_format)
        }
    }

    fn ready_secret_backend() -> SecretBackend {
        SecretBackend::with_adapter(Arc::new(TestSecretBackendAdapter))
    }

    fn insert_evm_wallet_with_secret(
        stored_secret: StoredSecret,
    ) -> (WalletInfo, DatabaseBackedKeystore) {
        let db = Database::new(":memory:").unwrap();
        let wallet = db
            .insert_evm_wallet_with_secret(
                "EVM Wallet".to_string(),
                "private-key".to_string(),
                "0x1234".to_string(),
                stored_secret,
                "private-key".to_string(),
            )
            .unwrap();

        (wallet, DatabaseBackedKeystore { db })
    }

    fn insert_global_evm_wallet_with_secret(stored_secret: StoredSecret) -> WalletInfo {
        let unique = Uuid::new_v4().simple().to_string();
        let suffix = &unique[..40.min(unique.len())];
        let address = format!("0x{:0<40}", suffix);

        let wallet = {
            let db = DB.lock().unwrap();
            db.insert_evm_wallet_with_secret(
                format!("EVM Command {unique}"),
                "private-key".to_string(),
                address,
                stored_secret,
                "private-key".to_string(),
            )
            .unwrap()
        };

        wallet
    }

    fn cleanup_global_evm_wallet(wallet_id: &str) {
        let db = DB.lock().unwrap();
        let _ = db.delete_evm_wallet(wallet_id);
    }

    fn test_wallet(wallet_type: &str) -> WalletInfo {
        WalletInfo {
            id: "wallet-id".to_string(),
            label: "EVM Wallet".to_string(),
            wallet_type: wallet_type.to_string(),
            address: "0x1234".to_string(),
            balance: 0.0,
            created_at: "2026-04-19T00:00:00Z".to_string(),
            updated_at: "2026-04-19T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn send_signing_returns_locked_without_keystore_access() {
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let secret_backend = ready_secret_backend();

        assert!(matches!(
            load_signing_secret(
                &test_wallet("mnemonic"),
                &secret_backend,
                &PanicKeystore,
                &session,
                SignerOperation::Send
            ),
            Err(SecurityError::Locked)
        ));
    }

    #[test]
    fn approve_signing_returns_locked_without_keystore_access() {
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let secret_backend = ready_secret_backend();

        assert!(matches!(
            load_signing_secret(
                &test_wallet("mnemonic"),
                &secret_backend,
                &PanicKeystore,
                &session,
                SignerOperation::Approve
            ),
            Err(SecurityError::Locked)
        ));
    }

    #[test]
    fn approve_signing_returns_expired_without_keystore_access() {
        let session = SessionManager::new(Duration::from_millis(1), Duration::from_secs(90));
        let secret_backend = ready_secret_backend();
        session
            .authorize_verified_operation(SignerOperation::Approve)
            .unwrap();
        std::thread::sleep(Duration::from_millis(5));

        assert!(matches!(
            load_signing_secret(
                &test_wallet("mnemonic"),
                &secret_backend,
                &PanicKeystore,
                &session,
                SignerOperation::Approve
            ),
            Err(SecurityError::Expired)
        ));
    }

    #[test]
    fn approve_signing_reads_plaintext_secret_row_after_unlock() {
        let (wallet, keystore) = insert_evm_wallet_with_secret(StoredSecret {
            secret_data: "0x59c6995e998f97a5a0044966f094538c5f1f6f67cb5a1f2f4c8f5d4f9b3c1d2e"
                .to_string(),
            secret_format: SECRET_FORMAT_PLAINTEXT_V0.to_string(),
        });
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let secret_backend = ready_secret_backend();
        session
            .authorize_verified_operation(SignerOperation::Approve)
            .unwrap();

        let result = load_signing_secret(
            &wallet,
            &secret_backend,
            &keystore,
            &session,
            SignerOperation::Approve,
        )
        .unwrap();

        assert!(matches!(
            result,
            Some(EvmSigningSecret::PrivateKey(secret))
                if secret == "0x59c6995e998f97a5a0044966f094538c5f1f6f67cb5a1f2f4c8f5d4f9b3c1d2e"
        ));
    }

    #[test]
    fn approve_signing_reads_migrated_secret_row_after_unlock() {
        let (wallet, keystore) = insert_evm_wallet_with_secret(
            encrypt_secret("0x59c6995e998f97a5a0044966f094538c5f1f6f67cb5a1f2f4c8f5d4f9b3c1d2e")
                .unwrap(),
        );
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let secret_backend = ready_secret_backend();
        session
            .authorize_verified_operation(SignerOperation::Approve)
            .unwrap();

        let result = load_signing_secret(
            &wallet,
            &secret_backend,
            &keystore,
            &session,
            SignerOperation::Approve,
        )
        .unwrap();

        assert!(matches!(
            result,
            Some(EvmSigningSecret::PrivateKey(secret))
                if secret == "0x59c6995e998f97a5a0044966f094538c5f1f6f67cb5a1f2f4c8f5d4f9b3c1d2e"
        ));
    }

    #[tokio::test]
    async fn approve_command_path_reads_plaintext_secret_row_before_spender_validation_failure() {
        let wallet = insert_global_evm_wallet_with_secret(StoredSecret {
            secret_data: "0x59c6995e998f97a5a0044966f094538c5f1f6f67cb5a1f2f4c8f5d4f9b3c1d2e"
                .to_string(),
            secret_format: SECRET_FORMAT_PLAINTEXT_V0.to_string(),
        });
        let secret_backend = Arc::new(SecretBackend::with_adapter(Arc::new(
            TestSecretBackendAdapter,
        )));
        let keystore = SqliteKeystore::new(&DB, secret_backend);
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        session
            .authorize_verified_operation(SignerOperation::Approve)
            .unwrap();

        let result = approve_erc20_token(
            wallet.id.clone(),
            1,
            "0x0000000000000000000000000000000000000001".to_string(),
            "not-an-address".to_string(),
            "1".to_string(),
            &ready_secret_backend(),
            &keystore,
            &session,
        )
        .await;

        cleanup_global_evm_wallet(&wallet.id);

        assert!(matches!(result, Err(message) if message.starts_with("Invalid spender address:")));
    }

    #[tokio::test]
    async fn approve_command_path_reads_migrated_secret_row_before_spender_validation_failure() {
        let wallet = insert_global_evm_wallet_with_secret(
            encrypt_secret("0x59c6995e998f97a5a0044966f094538c5f1f6f67cb5a1f2f4c8f5d4f9b3c1d2e")
                .unwrap(),
        );
        let secret_backend = Arc::new(SecretBackend::with_adapter(Arc::new(
            TestSecretBackendAdapter,
        )));
        let keystore = SqliteKeystore::new(&DB, secret_backend);
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        session
            .authorize_verified_operation(SignerOperation::Approve)
            .unwrap();

        let result = approve_erc20_token(
            wallet.id.clone(),
            1,
            "0x0000000000000000000000000000000000000001".to_string(),
            "not-an-address".to_string(),
            "1".to_string(),
            &ready_secret_backend(),
            &keystore,
            &session,
        )
        .await;

        cleanup_global_evm_wallet(&wallet.id);

        assert!(matches!(result, Err(message) if message.starts_with("Invalid spender address:")));
    }
}
