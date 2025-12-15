use crate::wallet::transaction_types::{
    BitcoinTransaction, SendBitcoinRequest, SendTransactionResponse,
    TransactionStatus, TransactionType,
};
use crate::DB;
use bdk::blockchain::{Blockchain, ElectrumBlockchain, GetHeight};
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client;
use bdk::psbt::PsbtUtils;
use bdk::{FeeRate, SignOptions, SyncOptions, Wallet};
use bdk::bitcoin::Network;
use chrono::Utc;
use std::str::FromStr;
use uuid::Uuid;

/// Fetch Bitcoin transaction history from the blockchain
pub async fn fetch_bitcoin_transaction_history(
    wallet_id: String,
    address: String,
) -> Result<Vec<BitcoinTransaction>, String> {
    // Get wallet secret to reconstruct wallet
    let (secret_data, secret_type) = {
        let db = DB.lock().unwrap();
        db.get_wallet_secret(&wallet_id)
            .map_err(|e| format!("Failed to get wallet secret: {}", e))?
            .ok_or_else(|| "Wallet secret not found".to_string())?
    };

    // Reconstruct the wallet
    let wallet = match secret_type.as_str() {
        "mnemonic" => {
            let descriptor = format!("wpkh({}/84'/0'/0'/0/*)", secret_data);
            Wallet::new(&descriptor, None, Network::Bitcoin, MemoryDatabase::default())
                .map_err(|e| format!("Failed to create wallet: {}", e))?
        }
        "private_key" => {
            let descriptor = format!("wpkh({})", secret_data);
            Wallet::new(&descriptor, None, Network::Bitcoin, MemoryDatabase::default())
                .map_err(|e| format!("Failed to create wallet: {}", e))?
        }
        _ => return Err("Unknown secret type".to_string()),
    };

    // Connect to Electrum server
    let client = Client::new("ssl://electrum.blockstream.info:50002")
        .map_err(|e| format!("Failed to connect to Electrum: {}", e))?;
    let blockchain = ElectrumBlockchain::from(client);

    // Sync wallet
    wallet
        .sync(&blockchain, SyncOptions::default())
        .map_err(|e| format!("Failed to sync wallet: {}", e))?;

    // Get all transactions
    let transactions = wallet.list_transactions(false)
        .map_err(|e| format!("Failed to list transactions: {}", e))?;

    let mut result = Vec::new();

    for tx_details in transactions {
        let tx_hash = tx_details.txid.to_string();

        // Determine transaction type
        let (tx_type, from_address, to_address) = if tx_details.received > tx_details.sent {
            // Receiving transaction
            (
                TransactionType::Receive,
                "Unknown".to_string(),
                address.clone(),
            )
        } else {
            // Sending transaction
            (
                TransactionType::Send,
                address.clone(),
                "Unknown".to_string(),
            )
        };

        let amount = if tx_type == TransactionType::Receive {
            (tx_details.received as f64) / 100_000_000.0
        } else {
            (tx_details.sent as f64) / 100_000_000.0
        };

        let fee = (tx_details.fee.unwrap_or(0) as f64) / 100_000_000.0;

        let status = if tx_details.confirmation_time.is_some() {
            TransactionStatus::Confirmed
        } else {
            TransactionStatus::Pending
        };

        let confirmations = if let Some(ref conf_time) = tx_details.confirmation_time {
            // Get current block height and calculate confirmations
            blockchain.get_height()
                .map(|height| height.saturating_sub(conf_time.height))
                .unwrap_or(0)
        } else {
            0
        };

        let block_height = tx_details.confirmation_time.as_ref().map(|ct| ct.height);

        let timestamp = if let Some(ref conf_time) = tx_details.confirmation_time {
            chrono::DateTime::from_timestamp(conf_time.timestamp as i64, 0)
                .unwrap_or_else(|| Utc::now())
                .to_rfc3339()
        } else {
            Utc::now().to_rfc3339()
        };

        let tx = BitcoinTransaction {
            id: Uuid::new_v4().to_string(),
            wallet_id: wallet_id.clone(),
            tx_hash,
            tx_type,
            from_address,
            to_address,
            amount,
            fee,
            status,
            confirmations,
            block_height,
            timestamp: timestamp.clone(),
            created_at: timestamp,
        };

        // Save to database
        {
            let db = DB.lock().unwrap();
            db.add_bitcoin_transaction(&tx)
                .map_err(|e| format!("Failed to save transaction: {}", e))?;
        }

        result.push(tx);
    }

    Ok(result)
}

/// Send Bitcoin transaction
pub async fn send_bitcoin_transaction(
    request: SendBitcoinRequest,
) -> Result<SendTransactionResponse, String> {
    // Get wallet secret
    let (secret_data, secret_type) = {
        let db = DB.lock().unwrap();
        db.get_wallet_secret(&request.wallet_id)
            .map_err(|e| format!("Failed to get wallet secret: {}", e))?
            .ok_or_else(|| "Wallet secret not found".to_string())?
    };

    // Get wallet info
    let wallet_info = {
        let db = DB.lock().unwrap();
        db.get_bitcoin_wallet(&request.wallet_id)
            .map_err(|e| format!("Failed to get wallet info: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };

    // Reconstruct the wallet
    let wallet = match secret_type.as_str() {
        "mnemonic" => {
            let descriptor = format!("wpkh({}/84'/0'/0'/0/*)", secret_data);
            Wallet::new(&descriptor, None, Network::Bitcoin, MemoryDatabase::default())
                .map_err(|e| format!("Failed to create wallet: {}", e))?
        }
        "private_key" => {
            let descriptor = format!("wpkh({})", secret_data);
            Wallet::new(&descriptor, None, Network::Bitcoin, MemoryDatabase::default())
                .map_err(|e| format!("Failed to create wallet: {}", e))?
        }
        _ => return Err("Unknown secret type".to_string()),
    };

    // Connect to Electrum server
    let client = Client::new("ssl://electrum.blockstream.info:50002")
        .map_err(|e| format!("Failed to connect to Electrum: {}", e))?;
    let blockchain = ElectrumBlockchain::from(client);

    // Sync wallet
    wallet
        .sync(&blockchain, SyncOptions::default())
        .map_err(|e| format!("Failed to sync wallet: {}", e))?;

    // Parse recipient address
    let recipient = bdk::bitcoin::Address::from_str(&request.to_address)
        .map_err(|e| format!("Invalid recipient address: {}", e))?;

    // Convert BTC to satoshis
    let amount_satoshis = (request.amount * 100_000_000.0) as u64;

    // Build transaction
    let mut tx_builder = wallet.build_tx();
    tx_builder.add_recipient(recipient.payload.script_pubkey(), amount_satoshis);

    // Set fee rate if provided
    if let Some(fee_rate) = request.fee_rate {
        tx_builder.fee_rate(FeeRate::from_sat_per_vb(fee_rate as f32));
    }

    let (mut psbt, _) = tx_builder
        .finish()
        .map_err(|e| format!("Failed to build transaction: {}", e))?;

    // Calculate fee before extracting tx (psbt moves after extract_tx)
    let fee = if let Some(fee_satoshis) = psbt.fee_amount() {
        (fee_satoshis as f64) / 100_000_000.0
    } else {
        0.0
    };

    // Sign transaction
    wallet
        .sign(&mut psbt, SignOptions::default())
        .map_err(|e| format!("Failed to sign transaction: {}", e))?;

    // Extract and broadcast transaction
    let tx = psbt.extract_tx();
    let tx_hash = tx.txid().to_string();

    blockchain
        .broadcast(&tx)
        .map_err(|e| format!("Failed to broadcast transaction: {}", e))?;

    // Save transaction to database
    let now = Utc::now().to_rfc3339();
    let tx_record = BitcoinTransaction {
        id: Uuid::new_v4().to_string(),
        wallet_id: request.wallet_id.clone(),
        tx_hash: tx_hash.clone(),
        tx_type: TransactionType::Send,
        from_address: wallet_info.address.clone(),
        to_address: request.to_address,
        amount: request.amount,
        fee,
        status: TransactionStatus::Pending,
        confirmations: 0,
        block_height: None,
        timestamp: now.clone(),
        created_at: now,
    };

    {
        let db = DB.lock().unwrap();
        db.add_bitcoin_transaction(&tx_record)
            .map_err(|e| format!("Failed to save transaction: {}", e))?;
    }

    Ok(SendTransactionResponse {
        tx_hash,
        message: "Transaction sent successfully".to_string(),
    })
}
