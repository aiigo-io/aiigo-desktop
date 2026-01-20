use crate::wallet::transaction_types::{
    EvmTransaction, SendEvmRequest, SendTransactionResponse,
    TransactionStatus, TransactionType,
};
use crate::wallet::evm::config::get_chain_by_id;
use crate::DB;
use chrono::Utc;
use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use ethers::types::{Address as EthAddress, TransactionReceipt, H256, U256};
use ethers::signers::coins_bip39::English;
use ethers::signers::MnemonicBuilder;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

const ETHERSCAN_API_KEY: &str = "TKG5YYYPSX97W8HG7ZHA319UXKWMXFKKG6";

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

/// Fetch EVM transaction history from the blockchain using Etherscan API
pub async fn fetch_evm_transaction_history(
    wallet_id: String,
    address: String,
    chain: String,
    chain_id: u64,
) -> Result<Vec<EvmTransaction>, String> {
    let api_url = get_etherscan_api_url(chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported for Etherscan API", chain_id))?;

    let chain_config = get_chain_by_id(chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported", chain_id))?;

    let mut all_transactions = Vec::new();

    // Fetch normal transactions
    let normal_txs = fetch_normal_transactions(&api_url, &address, chain_id).await?;
    all_transactions.extend(normal_txs);

    // Fetch ERC20 token transfers
    let token_txs = fetch_token_transactions(&api_url, &address, chain_id).await?;
    all_transactions.extend(token_txs);

    // Convert to EvmTransaction and save to database
    let mut result = Vec::new();
    let native_symbol = chain_config.assets().first()
        .map(|a| a.symbol.clone())
        .unwrap_or_else(|| "ETH".to_string());

    for tx in all_transactions {
        let tx_type = if tx.from.to_lowercase() == address.to_lowercase() {
            TransactionType::Send
        } else {
            TransactionType::Receive
        };

        let status = if tx.is_error == "0" {
            TransactionStatus::Confirmed
        } else {
            TransactionStatus::Failed
        };

        let block_number = tx.block_number.parse::<u64>().ok();
        
        let timestamp_secs = tx.timestamp.parse::<i64>()
            .unwrap_or_else(|_| Utc::now().timestamp());
        let timestamp = chrono::DateTime::from_timestamp(timestamp_secs, 0)
            .unwrap_or_else(|| Utc::now())
            .to_rfc3339();

        // Determine if it's a token transfer or native transfer
        let (asset_symbol, asset_name, contract_address, decimals) = if !tx.contract_address.is_empty() {
            // ERC20 token transfer
            let symbol = tx.token_symbol.clone().unwrap_or_else(|| "UNKNOWN".to_string());
            let name = tx.token_name.clone().unwrap_or_else(|| "Unknown Token".to_string());
            let decimals = tx.token_decimal.as_ref()
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
        return Err(format!("Etherscan API error: {}", etherscan_response.message));
    }

    let transactions: Vec<EtherscanTransaction> = serde_json::from_value(etherscan_response.result.clone())
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
        return Err(format!("Etherscan API error: {}", etherscan_response.message));
    }

    let transactions: Vec<EtherscanTransaction> = serde_json::from_value(etherscan_response.result.clone())
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
    let amount_wei = U256::from_dec_str(&request.amount)
        .map_err(|e| format!("Invalid amount: {}", e))?;

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
) -> Result<SendTransactionResponse, String> {
    // Get wallet secret
    let (secret_data, secret_type) = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet_secret(&request.wallet_id)
            .map_err(|e| format!("Failed to get wallet secret: {}", e))?
            .ok_or_else(|| "Wallet secret not found".to_string())?
    };

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
    let provider = Arc::new(provider);

    // Create wallet from secret
    let wallet = match secret_type.as_str() {
        "mnemonic" => {
            MnemonicBuilder::<English>::default()
                .phrase(secret_data.as_str())
                .derivation_path("m/44'/60'/0'/0/0")
                .map_err(|e| format!("Failed to set derivation path: {}", e))?
                .build()
                .map_err(|e| format!("Failed to build wallet: {}", e))?
                .with_chain_id(request.chain_id)
        }
        "private-key" | "private_key" => {
            let trimmed = secret_data.trim();
            let pk = if trimmed.starts_with("0x") {
                trimmed.to_string()
            } else {
                format!("0x{}", trimmed)
            };
            pk.parse::<LocalWallet>()
                .map_err(|e| format!("Failed to parse private key: {}", e))?
                .with_chain_id(request.chain_id)
        }
        _ => return Err(format!("Unsupported secret type: {}", secret_type)),
    };

    let client = SignerMiddleware::new(provider.clone(), wallet);

    // Parse recipient address
    let to_address = EthAddress::from_str(&request.to_address)
        .map_err(|e| format!("Invalid recipient address: {}", e))?;

    // Parse amount
    let amount_wei = U256::from_dec_str(&request.amount)
        .map_err(|e| format!("Invalid amount: {}", e))?;

    let tx_hash: H256;
    let gas_used_str: String;
    let gas_price_str: String;
    let fee: f64;
    let _status: TransactionStatus;
    // Set status to confirmed for now (mocking immediate confirmation)
    _status = TransactionStatus::Confirmed;

    // Check if it's a native token or ERC20 transfer
    if request.contract_address.is_none() {
        // Native token transfer (ETH, etc.)
        let mut tx = TransactionRequest::new()
            .to(to_address)
            .value(amount_wei);

        // Set gas limit if provided
        if let Some(gas_limit) = request.gas_limit {
            tx = tx.gas(U256::from(gas_limit));
        }

        // Set gas price if provided
        if let Some(gas_price) = request.gas_price {
            let gas_price_wei = U256::from_dec_str(&gas_price)
                .map_err(|e| format!("Invalid gas price: {}", e))?;
            tx = tx.gas_price(gas_price_wei);
        }

        // Send transaction
        let pending_tx = client
            .send_transaction(tx, None)
            .await
            .map_err(|e| format!("Failed to send transaction: {}", e))?;

        tx_hash = *pending_tx;
        gas_used_str = "0".to_string();
        gas_price_str = "0".to_string();
        fee = 0.0;
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

        let contract = Contract::new(contract_address, abi, Arc::new(client.clone()));

        // Call transfer function
        let call = contract
            .method::<_, bool>("transfer", (to_address, amount_wei))
            .map_err(|e| format!("Failed to create contract call: {}", e))?;

        let pending_tx = call
            .send()
            .await
            .map_err(|e| format!("Failed to send transaction: {}", e))?;

        tx_hash = *pending_tx;
        gas_used_str = "0".to_string();
        gas_price_str = "0".to_string();
        fee = 0.0;
    }

    // Find decimals for the asset
    let decimals = if let Some(addr) = &request.contract_address {
        chain_config.assets().iter()
            .find(|a| a.contract_address.as_ref().map(|ca| ca.to_lowercase()) == Some(addr.to_lowercase()))
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
        chain_config.assets().iter()
            .find(|a| a.contract_address.as_ref().map(|ca| ca.to_lowercase()) == Some(addr.to_lowercase()))
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
        status: TransactionStatus::Confirmed,
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

/// Helper to get transaction receipt
#[allow(dead_code)]
pub async fn get_transaction_receipt(
    tx_hash: String,
    chain_id: u64,
) -> Result<Option<TransactionReceipt>, String> {
    // Get chain config
    let chain_config = get_chain_by_id(chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported", chain_id))?;

    // Create provider
    let provider = Provider::<Http>::try_from(chain_config.rpc_url())
        .map_err(|e| format!("Failed to create provider: {}", e))?;

    // Parse transaction hash
    let hash = H256::from_str(&tx_hash)
        .map_err(|e| format!("Invalid transaction hash: {}", e))?;

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
) -> Result<SendTransactionResponse, String> {
    // Get wallet secret
    let (secret_data, secret_type) = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet_secret(&request.wallet_id)
            .map_err(|e| format!("Failed to get wallet secret: {}", e))?
            .ok_or_else(|| "Wallet secret not found".to_string())?
    };

    // Get wallet info
    let _wallet_info = {
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
    let provider = Arc::new(provider);

    // Create wallet from secret
    let wallet = match secret_type.as_str() {
        "mnemonic" => {
            MnemonicBuilder::<English>::default()
                .phrase(secret_data.as_str())
                .derivation_path("m/44'/60'/0'/0/0")
                .map_err(|e| format!("Failed to set derivation path: {}", e))?
                .build()
                .map_err(|e| format!("Failed to build wallet: {}", e))?
                .with_chain_id(request.chain_id)
        }
        "private_key" | "private-key" => {
            let trimmed = secret_data.trim();
            let pk = if trimmed.starts_with("0x") {
                trimmed.to_string()
            } else {
                format!("0x{}", trimmed)
            };
            pk.parse::<LocalWallet>()
                .map_err(|e| format!("Failed to parse private key: {}", e))?
                .with_chain_id(request.chain_id)
        }
        _ => return Err(format!("Unsupported secret type: {}", secret_type)),
    };

    let client = SignerMiddleware::new(provider.clone(), wallet);

    // Parse recipient address
    let to_address = EthAddress::from_str(&request.to)
        .map_err(|e| format!("Invalid recipient address: {}", e))?;

    // Parse value (amount of native token to send)
    let value = U256::from_dec_str(&request.value)
        .map_err(|e| format!("Invalid value: {}", e))?;

    // Parse gas limit
    let gas_limit = U256::from_dec_str(&request.gas_limit)
        .map_err(|e| format!("Invalid gas limit: {}", e))?;

    // Parse gas price (in wei)
    let gas_price = U256::from_dec_str(&request.gas_price)
        .map_err(|e| format!("Invalid gas price: {}", e))?;

    // Parse data (hex string)
    let data = if request.data.starts_with("0x") {
        hex::decode(&request.data[2..])
            .map_err(|e| format!("Invalid data hex: {}", e))?
    } else {
        hex::decode(&request.data)
            .map_err(|e| format!("Invalid data hex: {}", e))?
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
) -> Result<SendTransactionResponse, String> {
    // Get wallet secret
    let (secret_data, secret_type) = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet_secret(&wallet_id)
            .map_err(|e| format!("Failed to get wallet secret: {}", e))?
            .ok_or_else(|| "Wallet secret not found".to_string())?
    };

    // Get chain config
    let chain_config = get_chain_by_id(chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported", chain_id))?;

    // Create provider
    let provider = Provider::<Http>::try_from(chain_config.rpc_url())
        .map_err(|e| format!("Failed to create provider: {}", e))?;
    let provider = Arc::new(provider);

    // Create wallet from secret
    let wallet = match secret_type.as_str() {
        "mnemonic" => {
            MnemonicBuilder::<English>::default()
                .phrase(secret_data.as_str())
                .derivation_path("m/44'/60'/0'/0/0")
                .map_err(|e| format!("Failed to set derivation path: {}", e))?
                .build()
                .map_err(|e| format!("Failed to build wallet: {}", e))?
                .with_chain_id(chain_id)
        }
        "private-key" | "private_key" => {
            let trimmed = secret_data.trim();
            let pk = if trimmed.starts_with("0x") {
                trimmed.to_string()
            } else {
                format!("0x{}", trimmed)
            };
            pk.parse::<LocalWallet>()
                .map_err(|e| format!("Failed to parse private key: {}", e))?
                .with_chain_id(chain_id)
        }
        _ => return Err(format!("Unsupported secret type: {}", secret_type)),
    };

    let client = SignerMiddleware::new(provider.clone(), wallet);

    // Parse token contract address
    let contract_address = EthAddress::from_str(&token_address)
        .map_err(|e| format!("Invalid token address: {}", e))?;

    // Parse amount to approve (handle both hex and decimal formats)
    let approve_amount = if amount.starts_with("0x") || amount.starts_with("0X") {
        // Hex format (e.g., "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
        U256::from_str_radix(&amount[2..], 16)
            .map_err(|e| format!("Invalid hex amount: {}", e))?
    } else {
        // Decimal format
        U256::from_dec_str(&amount)
            .map_err(|e| format!("Invalid decimal amount: {}", e))?
    };

    // ERC20 ABI for approve function
    let abi = ethers::abi::parse_abi(&[
        "function approve(address spender, uint256 amount) returns (bool)",
    ])
    .map_err(|e| format!("Failed to parse ABI: {}", e))?;

    let contract = Contract::new(contract_address, abi, Arc::new(client.clone()));

    // Parse spender address from parameter (OpenOcean router address)
    let spender = EthAddress::from_str(&spender_address)
        .map_err(|e| format!("Invalid spender address: {}", e))?;

    // Call approve function
    let call = contract
        .method::<_, bool>("approve", (spender, approve_amount))
        .map_err(|e| format!("Failed to create approve call: {}", e))?;

    let pending_tx = call
        .send()
        .await
        .map_err(|e| format!("Failed to send approval transaction: {}", e))?;

    let tx_hash = *pending_tx;
    let tx_hash_str = format!("{:?}", tx_hash);

    // Wait for confirmation
    let _receipt = pending_tx
        .await
        .map_err(|e| format!("Failed to get receipt: {}", e))?
        .ok_or_else(|| "Approval transaction failed".to_string())?;

    Ok(SendTransactionResponse {
        tx_hash: tx_hash_str,
        message: "Token approved successfully".to_string(),
    })
}
