//! Frozen public contracts for the security subsystem.
//!
//! These are the minimal public security contracts still used by the current
//! wallet MVP. Change them deliberately alongside the active command surface
//! and frontend callers.
//! Reference: docs/architecture/executable-wallet-runtime-blueprint.md

use serde::{Deserialize, Serialize};

/// The operation a signer authority check is gating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignerOperation {
    Send,
    Approve,
    ExportMnemonic,
    ExportPrivateKey,
}

/// Structured denial reasons returned from the MVP security boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityError {
    Locked,
    PolicyDenied,
    OperationNotAllowed,
    UnknownWallet,
}
