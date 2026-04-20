pub use crate::wallet::sync::types::LifecycleStatus as TransactionStatus;

use serde::{Deserialize, Serialize};

/// Transaction type (send or receive)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Send,
    Receive,
    Approve,
    Contract,
}

impl TransactionType {
    pub fn as_str(&self) -> &str {
        match self {
            TransactionType::Send => "send",
            TransactionType::Receive => "receive",
            TransactionType::Approve => "approve",
            TransactionType::Contract => "contract",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "send" => TransactionType::Send,
            "receive" => TransactionType::Receive,
            "approve" => TransactionType::Approve,
            "contract" => TransactionType::Contract,
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
    pub send_all: Option<bool>,
}

/// Request to send EVM transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEvmRequest {
    pub wallet_id: String,
    pub to_address: String,
    pub amount: String, // in base units (e.g., wei as string)
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

/// Response for EVM gas estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmGasEstimationResponse {
    pub gas_limit: u64,
    pub gas_price: String, // in wei as string
}

/// Response for Bitcoin fee estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinFeeEstimationResponse {
    pub fast: f32,
    pub half_hour: f32,
    pub hour: f32,
}

/// Request to send a raw EVM transaction (for OpenOcean swaps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTransactionRequest {
    pub wallet_id: String,
    pub chain_id: u64,
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas_limit: String,
    pub gas_price: String,
}

#[cfg(test)]
mod tests {
    use super::TransactionStatus;

    #[test]
    fn transaction_status_from_str_accepts_phase3_and_legacy_values() {
        assert_eq!(TransactionStatus::from_str("broadcasted"), TransactionStatus::Broadcasted);
        assert_eq!(TransactionStatus::from_str("pending"), TransactionStatus::Pending);
        assert_eq!(TransactionStatus::from_str("confirmed"), TransactionStatus::Confirmed);
        assert_eq!(TransactionStatus::from_str("failed"), TransactionStatus::Failed);
        assert_eq!(TransactionStatus::from_str("replaced"), TransactionStatus::Replaced);
        assert_eq!(TransactionStatus::from_str("dropped"), TransactionStatus::Dropped);
        assert_eq!(TransactionStatus::from_str("unknown"), TransactionStatus::Pending);
    }

    #[test]
    fn transaction_status_broadcasted_does_not_equal_confirmed() {
        assert_ne!(TransactionStatus::after_broadcast(), TransactionStatus::Confirmed);
        assert_eq!(TransactionStatus::after_broadcast().as_str(), "broadcasted");
    }

    #[test]
    fn replaced_and_dropped_states_survive_round_trip_without_collapsing() {
        for status in [TransactionStatus::Replaced, TransactionStatus::Dropped] {
            let wire = serde_json::to_string(&status).unwrap();

            assert_eq!(TransactionStatus::from_str(status.as_str()), status);
            assert_eq!(serde_json::from_str::<TransactionStatus>(&wire).unwrap(), status);
            assert_ne!(status, TransactionStatus::Pending);
        }
    }
}
