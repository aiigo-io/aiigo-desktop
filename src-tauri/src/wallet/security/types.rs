//! Frozen public contracts for the security subsystem.
//!
//! These are the minimal public security contracts still used by the current
//! wallet MVP. Change them deliberately alongside the active command surface
//! and frontend callers.
//! Reference: docs/architecture/executable-wallet-runtime-blueprint.md

use serde::{Deserialize, Serialize};

pub const LOCAL_PASSWORD_IDLE_LOCK_SECONDS: u64 = 15 * 60;
pub const LOCAL_PASSWORD_REAUTH_WINDOW_SECONDS: u64 = 90;

/// The operation a signer authority check is gating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignerOperation {
    Send,
    Approve,
    ExportMnemonic,
    ExportPrivateKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalPasswordScope {
    PerInstallation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgotPasswordMode {
    ResetLocalDataAndRestoreWithRecoveryMaterial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalPasswordPolicy {
    pub installation_scope: LocalPasswordScope,
    pub device_only: bool,
    pub cloud_sync: bool,
    pub replaces_recovery_phrase: bool,
    pub requires_password_before_create_or_import: bool,
    pub requires_backend_ready_for_create_import_and_high_risk: bool,
    pub idle_lock_seconds: u64,
    pub lock_on_system_sleep: bool,
    pub high_risk_reauth_operations: Vec<SignerOperation>,
    pub forgot_password_mode: ForgotPasswordMode,
    pub reauth_window_seconds: u64,
}

impl LocalPasswordPolicy {
    pub fn wallet_mvp() -> Self {
        Self {
            installation_scope: LocalPasswordScope::PerInstallation,
            device_only: true,
            cloud_sync: false,
            replaces_recovery_phrase: false,
            requires_password_before_create_or_import: true,
            requires_backend_ready_for_create_import_and_high_risk: true,
            idle_lock_seconds: LOCAL_PASSWORD_IDLE_LOCK_SECONDS,
            lock_on_system_sleep: true,
            high_risk_reauth_operations: vec![
                SignerOperation::Send,
                SignerOperation::Approve,
                SignerOperation::ExportMnemonic,
                SignerOperation::ExportPrivateKey,
            ],
            forgot_password_mode: ForgotPasswordMode::ResetLocalDataAndRestoreWithRecoveryMaterial,
            reauth_window_seconds: LOCAL_PASSWORD_REAUTH_WINDOW_SECONDS,
        }
    }
}

/// Structured denial reasons returned from the MVP security boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityError {
    Locked,
    Expired,
    NoPassword,
    WrongPassword,
    ReauthRequired,
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
