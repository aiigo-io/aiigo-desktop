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
use serde::Deserialize;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

const REQUEST_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Deserialize)]
struct BlockstreamTx {
    txid: String,
    version: i32,
    locktime: u64,
    vin: Vec<BlockstreamVin>,
    vout: Vec<BlockstreamVout>,
    size: u64,
    weight: u64,
    fee: u64,
    status: BlockstreamTxStatus,
}

#[derive(Debug, Deserialize)]
struct BlockstreamVin {
    txid: String,
    vout: u32,
    prevout: Option<BlockstreamPrevout>,
    scriptsig: String,
    scriptsig_asm: String,
    #[serde(default)]
    witness: Vec<String>,
    is_coinbase: bool,
    sequence: u64,
}

#[derive(Debug, Deserialize)]
struct BlockstreamPrevout {
    scriptpubkey: String,
    scriptpubkey_asm: String,
    scriptpubkey_type: String,
    scriptpubkey_address: Option<String>,
    value: u64,
}

#[derive(Debug, Deserialize)]
struct BlockstreamVout {
    scriptpubkey: String,
    scriptpubkey_asm: String,
    scriptpubkey_type: String,
    scriptpubkey_address: Option<String>,
    value: u64,
}

#[derive(Debug, Deserialize)]
struct BlockstreamTxStatus {
    confirmed: bool,
    block_height: Option<u32>,
    block_hash: Option<String>,
    block_time: Option<u64>,
}

/// Fetch Bitcoin transaction history from Blockstream API
async fn fetch_transactions_from_blockstream(address: &str) -> Result<Vec<BlockstreamTx>, String> {
    let url = format!("https://blockstream.info/api/address/{}/txs", address);
    
    println!("[INFO] Fetching transactions from Blockstream API: {}", url);
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let transactions: Vec<BlockstreamTx> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    println!("[INFO] Successfully fetched {} transactions from Blockstream", transactions.len());
    
    Ok(transactions)
}

/// Fetch Bitcoin transaction history from the blockchain
pub async fn fetch_bitcoin_transaction_history(
    wallet_id: String,
    address: String,
) -> Result<Vec<BitcoinTransaction>, String> {
    println!("[INFO] Fetching Bitcoin transaction history for wallet: {}", wallet_id);
    println!("[INFO] Address: {}", address);
    
    // Fetch transactions from Blockstream API
    let blockstream_txs = fetch_transactions_from_blockstream(&address).await?;
    
    println!("[INFO] Found {} transactions from API", blockstream_txs.len());

    let mut result = Vec::new();

    // Get current block height from Blockstream API for calculating confirmations
    let current_height = get_current_block_height().await.unwrap_or(0);
    println!("[INFO] Current block height: {}", current_height);

    for (index, tx) in blockstream_txs.iter().enumerate() {
        let tx_hash = tx.txid.clone();
        println!("[INFO] Processing transaction {}/{}: {}", index + 1, blockstream_txs.len(), tx_hash);

        // Calculate total input and output values for this address
        let mut received: u64 = 0;
        let mut sent: u64 = 0;

        // Check outputs (vout) - money received to this address
        for vout in &tx.vout {
            if let Some(ref addr) = vout.scriptpubkey_address {
                if addr == &address {
                    received += vout.value;
                    println!("[INFO] Found output to our address: {} satoshis", vout.value);
                }
            }
        }

        // Check inputs (vin) - money sent from this address
        for vin in &tx.vin {
            if let Some(ref prevout) = vin.prevout {
                if let Some(ref addr) = prevout.scriptpubkey_address {
                    if addr == &address {
                        sent += prevout.value;
                        println!("[INFO] Found input from our address: {} satoshis", prevout.value);
                    }
                }
            }
        }

        // Determine transaction type
        let (tx_type, from_address, to_address) = if received > sent {
            // Receiving transaction
            println!("[INFO] Transaction type: Receive (received: {}, sent: {})", received, sent);
            (
                TransactionType::Receive,
                "Unknown".to_string(),
                address.clone(),
            )
        } else {
            // Sending transaction
            println!("[INFO] Transaction type: Send (received: {}, sent: {})", received, sent);
            (
                TransactionType::Send,
                address.clone(),
                "Unknown".to_string(),
            )
        };

        // Calculate net amount
        let amount = if tx_type == TransactionType::Receive {
            (received as f64) / 100_000_000.0
        } else {
            (sent as f64) / 100_000_000.0
        };

        let fee = (tx.fee as f64) / 100_000_000.0;

        let status = if tx.status.confirmed {
            TransactionStatus::Confirmed
        } else {
            TransactionStatus::Pending
        };

        let confirmations = if let Some(block_height) = tx.status.block_height {
            if current_height > 0 {
                current_height.saturating_sub(block_height)
            } else {
                0
            }
        } else {
            0
        };

        let timestamp = if let Some(block_time) = tx.status.block_time {
            chrono::DateTime::from_timestamp(block_time as i64, 0)
                .unwrap_or_else(|| Utc::now())
                .to_rfc3339()
        } else {
            Utc::now().to_rfc3339()
        };

        println!("[INFO] Transaction details - Amount: {} BTC, Fee: {} BTC, Status: {:?}, Confirmations: {}", 
                 amount, fee, status, confirmations);

        let tx_record = BitcoinTransaction {
            id: Uuid::new_v4().to_string(),
            wallet_id: wallet_id.clone(),
            tx_hash: tx_hash.clone(),
            tx_type,
            from_address,
            to_address,
            amount,
            fee,
            status,
            confirmations,
            block_height: tx.status.block_height,
            timestamp: timestamp.clone(),
            created_at: timestamp,
        };

        // Save to database
        {
            let db = DB.lock().unwrap();
            db.add_bitcoin_transaction(&tx_record)
                .map_err(|e| {
                    println!("[ERROR] Failed to save transaction {}: {}", tx_hash, e);
                    format!("Failed to save transaction: {}", e)
                })?;
        }

        println!("[SUCCESS] Transaction {} saved to database", tx_hash);
        result.push(tx_record);
    }

    println!("[SUCCESS] Fetched and saved {} Bitcoin transactions", result.len());
    Ok(result)
}

/// Get current block height from Blockstream API
async fn get_current_block_height() -> Result<u32, String> {
    let url = "https://blockstream.info/api/blocks/tip/height";
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let height: u32 = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse block height: {}", e))?;

    Ok(height)
}

/// Send Bitcoin transaction
pub async fn send_bitcoin_transaction(
    request: SendBitcoinRequest,
) -> Result<SendTransactionResponse, String> {
    println!("[INFO] Sending Bitcoin transaction from wallet: {}", request.wallet_id);
    println!("[INFO] Recipient: {}, Amount: {} BTC", request.to_address, request.amount);
    
    // Get wallet secret
    let (secret_data, secret_type) = {
        let db = DB.lock().unwrap();
        db.get_wallet_secret(&request.wallet_id)
            .map_err(|e| {
                println!("[ERROR] Failed to get wallet secret: {}", e);
                format!("Failed to get wallet secret: {}", e)
            })?
            .ok_or_else(|| {
                println!("[ERROR] Wallet secret not found");
                "Wallet secret not found".to_string()
            })?
    };

    // Get wallet info
    let wallet_info = {
        let db = DB.lock().unwrap();
        db.get_bitcoin_wallet(&request.wallet_id)
            .map_err(|e| {
                println!("[ERROR] Failed to get wallet info: {}", e);
                format!("Failed to get wallet info: {}", e)
            })?
            .ok_or_else(|| {
                println!("[ERROR] Wallet not found");
                "Wallet not found".to_string()
            })?
    };

    println!("[INFO] Wallet secret type: {}", secret_type);

    // Reconstruct the wallet
    let wallet = match secret_type.as_str() {
        "mnemonic" => {
            let descriptor = format!("wpkh({}/84'/0'/0'/0/*)", secret_data);
            println!("[INFO] Creating wallet with mnemonic descriptor");
            Wallet::new(&descriptor, None, Network::Bitcoin, MemoryDatabase::default())
                .map_err(|e| {
                    println!("[ERROR] Failed to create wallet: {}", e);
                    format!("Failed to create wallet: {}", e)
                })?
        }
        "private_key" | "private-key" => {
            let descriptor = format!("wpkh({})", secret_data);
            println!("[INFO] Creating wallet with private key descriptor");
            Wallet::new(&descriptor, None, Network::Bitcoin, MemoryDatabase::default())
                .map_err(|e| {
                    println!("[ERROR] Failed to create wallet: {}", e);
                    format!("Failed to create wallet: {}", e)
                })?
        }
        _ => {
            println!("[ERROR] Unknown secret type: {}", secret_type);
            return Err(format!("Unknown secret type: {}", secret_type));
        }
    };

    // Connect to Electrum server
    println!("[INFO] Connecting to Electrum server...");
    let client = Client::new("ssl://electrum.blockstream.info:50002")
        .map_err(|e| {
            println!("[ERROR] Failed to connect to Electrum: {}", e);
            format!("Failed to connect to Electrum: {}", e)
        })?;
    let blockchain = ElectrumBlockchain::from(client);
    println!("[INFO] Connected to Electrum server");

    // Sync wallet
    println!("[INFO] Syncing wallet...");
    wallet
        .sync(&blockchain, SyncOptions::default())
        .map_err(|e| {
            println!("[ERROR] Failed to sync wallet: {}", e);
            format!("Failed to sync wallet: {}", e)
        })?;
    println!("[INFO] Wallet synced successfully");

    // Parse recipient address
    println!("[INFO] Parsing recipient address...");
    let recipient = bdk::bitcoin::Address::from_str(&request.to_address)
        .map_err(|e| {
            println!("[ERROR] Invalid recipient address: {}", e);
            format!("Invalid recipient address: {}", e)
        })?;

    // Convert BTC to satoshis
    let amount_satoshis = (request.amount * 100_000_000.0) as u64;
    println!("[INFO] Amount in satoshis: {}", amount_satoshis);

    // Build transaction
    println!("[INFO] Building transaction...");
    let mut tx_builder = wallet.build_tx();
    tx_builder.add_recipient(recipient.payload.script_pubkey(), amount_satoshis);

    // Set fee rate if provided
    if let Some(fee_rate) = request.fee_rate {
        println!("[INFO] Using custom fee rate: {} sat/vB", fee_rate);
        tx_builder.fee_rate(FeeRate::from_sat_per_vb(fee_rate as f32));
    }

    let (mut psbt, _) = tx_builder
        .finish()
        .map_err(|e| {
            println!("[ERROR] Failed to build transaction: {}", e);
            format!("Failed to build transaction: {}", e)
        })?;
    println!("[INFO] Transaction built successfully");

    // Calculate fee before extracting tx (psbt moves after extract_tx)
    let fee = if let Some(fee_satoshis) = psbt.fee_amount() {
        (fee_satoshis as f64) / 100_000_000.0
    } else {
        0.0
    };
    println!("[INFO] Transaction fee: {} BTC", fee);

    // Sign transaction
    println!("[INFO] Signing transaction...");
    wallet
        .sign(&mut psbt, SignOptions::default())
        .map_err(|e| {
            println!("[ERROR] Failed to sign transaction: {}", e);
            format!("Failed to sign transaction: {}", e)
        })?;
    println!("[INFO] Transaction signed successfully");

    // Extract and broadcast transaction
    let tx = psbt.extract_tx();
    let tx_hash = tx.txid().to_string();
    println!("[INFO] Transaction hash: {}", tx_hash);

    println!("[INFO] Broadcasting transaction...");
    blockchain
        .broadcast(&tx)
        .map_err(|e| {
            println!("[ERROR] Failed to broadcast transaction: {}", e);
            format!("Failed to broadcast transaction: {}", e)
        })?;
    println!("[SUCCESS] Transaction broadcasted successfully");

    // Save transaction to database
    println!("[INFO] Saving transaction to database...");
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
            .map_err(|e| {
                println!("[ERROR] Failed to save transaction: {}", e);
                format!("Failed to save transaction: {}", e)
            })?;
    }

    println!("[SUCCESS] Transaction saved to database");
    println!("[SUCCESS] Bitcoin transaction completed: {}", tx_hash);

    Ok(SendTransactionResponse {
        tx_hash,
        message: "Transaction sent successfully".to_string(),
    })
}
