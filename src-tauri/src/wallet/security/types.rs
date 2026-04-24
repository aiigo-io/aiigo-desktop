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
    Expired,
    NoPassword,
    WrongPassword,
    PolicyDenied,
    OperationNotAllowed,
    UnknownWallet,
    SecretBackendUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordKdfParams {
    pub memory_cost_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordAuthState {
    pub password_hash: String,
    pub password_salt: String,
    pub kdf_params: PasswordKdfParams,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretBackendUnavailableKind {
    KeyringUnavailable,
    SecretServiceUnreachable,
    KeyDecodeFailed,
    AccessDenied,
    UnknownBackendError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretBackendUnavailableReason {
    pub kind: SecretBackendUnavailableKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretBackendStatus {
    Ready,
    Unavailable {
        reason: SecretBackendUnavailableReason,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SecretMigrationState {
    pub attempted_rows: usize,
    pub migrated_rows: usize,
    pub skipped_rows: usize,
    pub failed_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityBackendState {
    pub backend_status: SecretBackendStatus,
    pub migration: SecretMigrationState,
    pub has_legacy_plaintext_secrets: bool,
    pub degraded: bool,
}
