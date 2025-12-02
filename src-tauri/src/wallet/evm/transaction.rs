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
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

/// Fetch EVM transaction history from the blockchain
pub async fn fetch_evm_transaction_history(
    _wallet_id: String,
    address: String,
    _chain: String,
    chain_id: u64,
) -> Result<Vec<EvmTransaction>, String> {
    // Get chain config
    let chain_config = get_chain_by_id(chain_id)
        .ok_or_else(|| format!("Chain ID {} not supported", chain_id))?;

    // Create provider
    let provider = Provider::<Http>::try_from(chain_config.rpc_url())
        .map_err(|e| format!("Failed to create provider: {}", e))?;

    let eth_address = EthAddress::from_str(&address)
        .map_err(|e| format!("Invalid address: {}", e))?;

    // Get current block number
    let current_block = provider
        .get_block_number()
        .await
        .map_err(|e| format!("Failed to get block number: {}", e))?;

    // Fetch transactions (last 1000 blocks or less)
    let from_block = current_block.saturating_sub(1000u64.into());

    // Get transaction history using eth_getLogs (for incoming transactions)
    // Note: This is a simplified version. For production, you'd want to use
    // an indexer service like Etherscan API or The Graph

    let _filter = Filter::new()
        .address(eth_address)
        .from_block(from_block)
        .to_block(current_block);

    // For now, we'll return an empty list and implement a basic version
    // In production, you'd integrate with Etherscan API or similar service
    let transactions = Vec::new();

    // This is a placeholder - in production you'd want to:
    // 1. Use Etherscan API or similar service
    // 2. Query both incoming and outgoing transactions
    // 3. Handle ERC20 token transfers
    // 4. Parse transaction data properly

    Ok(transactions)
}

/// Send EVM transaction
pub async fn send_evm_transaction(
    request: SendEvmRequest,
) -> Result<SendTransactionResponse, String> {
    // Get wallet secret
    let (secret_data, _secret_type) = {
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

    // Create wallet from private key
    let wallet = secret_data
        .parse::<LocalWallet>()
        .map_err(|e| format!("Failed to parse private key: {}", e))?
        .with_chain_id(request.chain_id);

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

        // Wait for transaction to be mined
        let receipt = pending_tx
            .await
            .map_err(|e| format!("Failed to get receipt: {}", e))?
            .ok_or_else(|| "Transaction failed".to_string())?;

        gas_used_str = receipt.gas_used.unwrap_or_default().to_string();
        gas_price_str = receipt.effective_gas_price.unwrap_or_default().to_string();

        let gas_used_u256 = receipt.gas_used.unwrap_or_default();
        let gas_price_u256 = receipt.effective_gas_price.unwrap_or_default();
        let fee_wei = gas_used_u256 * gas_price_u256;

        // Convert to native token (assuming 18 decimals)
        fee = fee_wei.as_u128() as f64 / 1e18;
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

        // Wait for transaction to be mined
        let receipt = pending_tx
            .await
            .map_err(|e| format!("Failed to get receipt: {}", e))?
            .ok_or_else(|| "Transaction failed".to_string())?;

        gas_used_str = receipt.gas_used.unwrap_or_default().to_string();
        gas_price_str = receipt.effective_gas_price.unwrap_or_default().to_string();

        let gas_used_u256 = receipt.gas_used.unwrap_or_default();
        let gas_price_u256 = receipt.effective_gas_price.unwrap_or_default();
        let fee_wei = gas_used_u256 * gas_price_u256;

        // Convert to native token (assuming 18 decimals)
        fee = fee_wei.as_u128() as f64 / 1e18;
    }

    // Parse amount as float for display
    let amount_float = U256::from_dec_str(&request.amount)
        .map(|v| v.as_u128() as f64 / 1e18)
        .unwrap_or(0.0);

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
        asset_name: "".to_string(), // Could be fetched from contract
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
