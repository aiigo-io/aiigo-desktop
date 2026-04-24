//! Frozen public contracts for the sync engine and unified transaction lifecycle.
//!
//! These types define the current MVP sync and lifecycle vocabulary.
//! Reference: docs/architecture/executable-wallet-runtime-blueprint.md

use serde::{Deserialize, Serialize};

/// Default BTC finality threshold used by the current lifecycle mapping.
pub const BITCOIN_MIN_CONFIRMATIONS: u32 = 1;

/// Default EVM receipt-depth threshold used by the current lifecycle mapping.
pub const EVM_MIN_BLOCK_DEPTH: u64 = 1;

/// Unified transaction lifecycle vocabulary for BTC and EVM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    Broadcasted,
    Pending,
    Confirmed,
    Failed,
    Replaced,
    Dropped,
}

impl LifecycleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Broadcasted => "broadcasted",
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
            Self::Replaced => "replaced",
            Self::Dropped => "dropped",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "broadcasted" => Self::Broadcasted,
            "pending" => Self::Pending,
            "confirmed" => Self::Confirmed,
            "failed" => Self::Failed,
            "replaced" => Self::Replaced,
            "dropped" => Self::Dropped,
            _ => Self::Pending,
        }
    }

    pub fn after_broadcast() -> Self {
        Self::Broadcasted
    }

    pub fn from_bitcoin_confirmations(confirmations: u32, min_confirmations: u32) -> Self {
        if confirmations >= min_confirmations.max(1) {
            Self::Confirmed
        } else {
            Self::Pending
        }
    }

    pub fn from_evm_receipt(
        receipt_success: Option<bool>,
        block_depth: Option<u64>,
        min_block_depth: u64,
    ) -> Self {
        match receipt_success {
            Some(false) => Self::Failed,
            Some(true) if block_depth.unwrap_or(0) >= min_block_depth.max(1) => Self::Confirmed,
            Some(true) | None => Self::Pending,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Confirmed | Self::Failed | Self::Replaced | Self::Dropped)
    }
}

/// Reason a sync operation was triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncReason {
    Query,
    Manual,
    AppStart,
    AfterBroadcast,
    Periodic,
    ChainChange,
}

/// Logical surface a sync pass was targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTarget {
    BitcoinWalletBalance,
    EvmWalletBalances,
    Dashboard,
    BitcoinHistory,
    EvmHistory,
    TransactionLifecycle,
    ApprovalState,
}

/// Outcome of a sync pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOutcome {
    pub reason: SyncReason,
    pub target: SyncTarget,
    pub updated_at: Option<i64>,
    pub partial: bool,
    pub failed_sources: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{LifecycleStatus, BITCOIN_MIN_CONFIRMATIONS, EVM_MIN_BLOCK_DEPTH};

    #[test]
    fn transaction_status_round_trips_with_six_state_snake_case() {
        let statuses = [
            (LifecycleStatus::Broadcasted, "\"broadcasted\""),
            (LifecycleStatus::Pending, "\"pending\""),
            (LifecycleStatus::Confirmed, "\"confirmed\""),
            (LifecycleStatus::Failed, "\"failed\""),
            (LifecycleStatus::Replaced, "\"replaced\""),
            (LifecycleStatus::Dropped, "\"dropped\""),
        ];

        for (status, wire) in statuses {
            assert_eq!(serde_json::to_string(&status).unwrap(), wire);
            assert_eq!(serde_json::from_str::<LifecycleStatus>(wire).unwrap(), status);
        }
    }

    #[test]
    fn transaction_status_transition_helpers_follow_phase3_rules() {
        assert_eq!(LifecycleStatus::after_broadcast(), LifecycleStatus::Broadcasted);
        assert_eq!(
            LifecycleStatus::from_bitcoin_confirmations(0, BITCOIN_MIN_CONFIRMATIONS),
            LifecycleStatus::Pending
        );
        assert_eq!(
            LifecycleStatus::from_bitcoin_confirmations(1, BITCOIN_MIN_CONFIRMATIONS),
            LifecycleStatus::Confirmed
        );
        assert_eq!(
            LifecycleStatus::from_evm_receipt(Some(true), Some(0), EVM_MIN_BLOCK_DEPTH),
            LifecycleStatus::Pending
        );
        assert_eq!(
            LifecycleStatus::from_evm_receipt(Some(true), Some(1), EVM_MIN_BLOCK_DEPTH),
            LifecycleStatus::Confirmed
        );
        assert_eq!(
            LifecycleStatus::from_evm_receipt(Some(false), Some(1), EVM_MIN_BLOCK_DEPTH),
            LifecycleStatus::Failed
        );
        assert!(!LifecycleStatus::Broadcasted.is_terminal());
        assert!(LifecycleStatus::Confirmed.is_terminal());
    }
}
