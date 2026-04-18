//! Frozen public contracts for the sync engine and unified transaction lifecycle.
//!
//! `LifecycleStatus` is the 6-state vocabulary that will REPLACE the 3-state
//! `TransactionStatus` in `wallet/transaction_types.rs`. Phase 3 performs the
//! migration; until Phase 3 completes, both types coexist intentionally.
//! Gate 3 (scripts/check_task.sh) verifies the variant count is exactly six.

use serde::{Deserialize, Serialize};

/// Unified transaction lifecycle vocabulary for BTC and EVM.
/// Variant count is frozen at six; additions require plan update.
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

/// Reason a sync operation was triggered.
/// Phase 3 may extend this enum; existing variants must not be renamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncReason {
    Manual,
    AppStart,
    AfterBroadcast,
    Periodic,
    ChainChange,
}

/// Outcome of a sync pass. Body filled in Phase 3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOutcome {
    pub reason: SyncReason,
    pub updated_at: i64,
    pub failed_sources: Vec<String>,
}
