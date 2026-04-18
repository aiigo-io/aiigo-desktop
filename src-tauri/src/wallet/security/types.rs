//! Frozen public contracts for the security subsystem.
//!
//! These signatures are the ground truth referenced by Gate 2 and Gate 3
//! of the per-phase drift checks. Do NOT rename fields or add/remove
//! variants without first updating:
//!   - docs/superpowers/plans/2026-04-18-wallet-foundation-hardening.md
//!   - scripts/check_task.sh

use serde::{Deserialize, Serialize};

/// The operation a signer authority check is gating.
/// Plan reference: Phase 1 - Key Implementation Logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignerOperation {
    Send,
    Approve,
    ExportMnemonic,
    ExportPrivateKey,
}

/// Structured denial reasons returned from the security boundary.
/// Phase 1 may extend this enum; existing variants must not be renamed or removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityError {
    Locked,
    PolicyDenied,
    OperationNotAllowed,
    UnknownWallet,
}
