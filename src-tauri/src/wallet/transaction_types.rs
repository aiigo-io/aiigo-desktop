use serde::{Deserialize, Serialize};

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

impl TransactionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TransactionStatus::Pending => "pending",
            TransactionStatus::Confirmed => "confirmed",
            TransactionStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => TransactionStatus::Pending,
            "confirmed" => TransactionStatus::Confirmed,
            "failed" => TransactionStatus::Failed,
            _ => TransactionStatus::Pending,
        }
    }
}

/// Transaction type (send or receive)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Send,
    Receive,
}

impl TransactionType {
    pub fn as_str(&self) -> &str {
        match self {
            TransactionType::Send => "send",
            TransactionType::Receive => "receive",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "send" => TransactionType::Send,
            "receive" => TransactionType::Receive,
            _ => TransactionType::Receive,
        }
    }
}

/// Bitcoin transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub id: String,
    pub wallet_id: String,
    pub tx_hash: String,
    pub tx_type: TransactionType,
    pub from_address: String,
    pub to_address: String,
    pub amount: f64, // in BTC
    pub fee: f64, // in BTC
    pub status: TransactionStatus,
    pub confirmations: u32,
    pub block_height: Option<u32>,
    pub timestamp: String,
    pub created_at: String,
}

/// EVM transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmTransaction {
    pub id: String,
    pub wallet_id: String,
    pub tx_hash: String,
    pub tx_type: TransactionType,
    pub from_address: String,
    pub to_address: String,
    pub amount: String, // Store as string to preserve precision
    pub amount_float: f64, // For UI display
    pub asset_symbol: String,
    pub asset_name: String,
    pub contract_address: Option<String>, // None for native token
    pub chain: String,
    pub chain_id: u64,
    pub gas_used: String,
    pub gas_price: String,
    pub fee: f64, // Total fee in native token (e.g., ETH)
    pub status: TransactionStatus,
    pub block_number: Option<u64>,
    pub timestamp: String,
    pub created_at: String,
}

/// Request to send Bitcoin transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendBitcoinRequest {
    pub wallet_id: String,
    pub to_address: String,
    pub amount: f64, // in BTC
    pub fee_rate: Option<f64>, // satoshis per byte, optional
}

/// Request to send EVM transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEvmRequest {
    pub wallet_id: String,
    pub to_address: String,
    pub amount: String, // in token units (e.g., "1.5" for 1.5 ETH)
    pub chain: String,
    pub chain_id: u64,
    pub asset_symbol: String,
    pub contract_address: Option<String>, // None for native token
    pub gas_limit: Option<u64>,
    pub gas_price: Option<String>, // in gwei
}

/// Response after sending a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransactionResponse {
    pub tx_hash: String,
    pub message: String,
}
