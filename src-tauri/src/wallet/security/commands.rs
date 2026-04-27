use super::auth::{hash_password, verify_password};
use super::backend::SecretBackend;
use super::keystore::Keystore;
use super::secret_envelope::reset_master_key_after_local_data_reset;
use super::session::SessionManager;
use super::types::{
    LocalPasswordPolicy, SecretMigrationState, SecurityBackendState, SecurityError, SignerOperation,
};
use crate::DB;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Clone, Default)]
pub struct StartupSecurityState {
    pub migration: SecretMigrationState,
    pub has_legacy_plaintext_secrets: bool,
}

pub struct AppSecurity {
    pub session_manager: Arc<SessionManager>,
    pub keystore: Arc<dyn Keystore + Send + Sync>,
    pub secret_backend: Arc<SecretBackend>,
    pub startup_state: Arc<Mutex<StartupSecurityState>>,
}

impl AppSecurity {
    pub fn session_manager(&self) -> &SessionManager {
        self.session_manager.as_ref()
    }

    pub fn keystore(&self) -> &(dyn Keystore + Send + Sync) {
        self.keystore.as_ref()
    }

    pub fn secret_backend(&self) -> &SecretBackend {
        self.secret_backend.as_ref()
    }

    pub fn startup_state(&self) -> &Mutex<StartupSecurityState> {
        self.startup_state.as_ref()
    }
}

fn security_has_password_inner() -> Result<bool, SecurityError> {
    let db = DB.lock().map_err(|_| SecurityError::OperationNotAllowed)?;
    db.security_has_password()
        .map_err(|_| SecurityError::OperationNotAllowed)
}

fn security_setup_password_inner(password: &str) -> Result<(), SecurityError> {
    if security_has_password_inner()? {
        return Err(SecurityError::PolicyDenied);
    }

    let auth_state = hash_password(password)?;
    let db = DB.lock().map_err(|_| SecurityError::OperationNotAllowed)?;
    db.upsert_security_password_state(&auth_state)
        .map_err(|_| SecurityError::OperationNotAllowed)
}

fn security_unlock_inner(password: &str, state: &AppSecurity) -> Result<(), SecurityError> {
    let auth_state = load_password_auth_state()?;

    if !verify_password(password, &auth_state)? {
        state.session_manager().lock();
        return Err(SecurityError::WrongPassword);
    }

    state.session_manager().unlock_verified()
}

fn load_password_auth_state() -> Result<super::types::PasswordAuthState, SecurityError> {
    let db = DB.lock().map_err(|_| SecurityError::OperationNotAllowed)?;
    db.load_security_password_state()
        .map_err(|_| SecurityError::OperationNotAllowed)?
        .ok_or(SecurityError::NoPassword)
}

pub(crate) fn ensure_local_password_boundary_ready(
    state: &AppSecurity,
) -> Result<(), SecurityError> {
    if !security_has_password_inner()? {
        return Err(SecurityError::NoPassword);
    }

    state.secret_backend().ensure_ready_for_command()
}

pub(crate) fn ensure_local_password_configured() -> Result<(), SecurityError> {
    if !security_has_password_inner()? {
        return Err(SecurityError::NoPassword);
    }

    Ok(())
}

fn security_lock_inner(state: &AppSecurity) -> Result<(), SecurityError> {
    state.session_manager().lock();
    Ok(())
}

fn security_is_unlocked_inner(state: &AppSecurity) -> Result<bool, SecurityError> {
    Ok(state.session_manager().is_unlocked())
}

fn security_get_backend_state_inner(
    state: &AppSecurity,
) -> Result<SecurityBackendState, SecurityError> {
    let startup_state = state
        .startup_state()
        .lock()
        .map_err(|_| SecurityError::OperationNotAllowed)?
        .clone();
    let backend_status = state.secret_backend().current_status();
    let degraded = matches!(
        backend_status,
        super::types::SecretBackendStatus::Unavailable { .. }
    ) || startup_state.has_legacy_plaintext_secrets
        || startup_state.migration.failed_rows > 0;

    Ok(SecurityBackendState {
        backend_status,
        migration: startup_state.migration,
        has_legacy_plaintext_secrets: startup_state.has_legacy_plaintext_secrets,
        degraded,
    })
}

fn security_probe_backend_inner(
    state: &AppSecurity,
) -> Result<SecurityBackendState, SecurityError> {
    state.secret_backend().refresh_status();
    security_get_backend_state_inner(state)
}

fn security_get_local_password_policy_inner() -> LocalPasswordPolicy {
    LocalPasswordPolicy::wallet_mvp()
}

fn security_authorize_operation_inner(
    password: &str,
    operation: SignerOperation,
    state: &AppSecurity,
) -> Result<(), SecurityError> {
    let auth_state = load_password_auth_state()?;

    if !verify_password(password, &auth_state)? {
        state.session_manager().lock();
        return Err(SecurityError::WrongPassword);
    }

    state.secret_backend().ensure_ready_for_command()?;
    state
        .session_manager()
        .authorize_verified_operation(operation)
}

fn security_reset_local_wallet_data_inner(state: &AppSecurity) -> Result<(), SecurityError> {
    state.session_manager().lock();
    let db = DB.lock().map_err(|_| SecurityError::OperationNotAllowed)?;
    db.clear_local_wallet_data()
        .map_err(|_| SecurityError::OperationNotAllowed)?;
    drop(db);
    reset_master_key_after_local_data_reset().map_err(|_| SecurityError::OperationNotAllowed)?;

    if let Ok(mut startup_state) = state.startup_state().lock() {
        *startup_state = StartupSecurityState::default();
    }

    Ok(())
}

#[tauri::command]
pub async fn security_has_password() -> Result<bool, SecurityError> {
    security_has_password_inner()
}

#[tauri::command]
pub async fn security_setup_password(password: String) -> Result<(), SecurityError> {
    security_setup_password_inner(&password)
}

#[tauri::command]
pub async fn security_unlock(
    password: String,
    state: tauri::State<'_, AppSecurity>,
) -> Result<(), SecurityError> {
    security_unlock_inner(&password, &state)
}

#[tauri::command]
pub async fn security_lock(state: tauri::State<'_, AppSecurity>) -> Result<(), SecurityError> {
    security_lock_inner(&state)
}

#[tauri::command]
pub async fn security_is_unlocked(
    state: tauri::State<'_, AppSecurity>,
) -> Result<bool, SecurityError> {
    security_is_unlocked_inner(&state)
}

#[tauri::command]
pub async fn security_get_backend_state(
    state: tauri::State<'_, AppSecurity>,
) -> Result<SecurityBackendState, SecurityError> {
    security_get_backend_state_inner(&state)
}

#[tauri::command]
pub async fn security_probe_backend(
    state: tauri::State<'_, AppSecurity>,
) -> Result<SecurityBackendState, SecurityError> {
    security_probe_backend_inner(&state)
}

#[tauri::command]
pub async fn security_get_local_password_policy() -> Result<LocalPasswordPolicy, SecurityError> {
    Ok(security_get_local_password_policy_inner())
}

#[tauri::command]
pub async fn security_authorize_operation(
    password: String,
    operation: SignerOperation,
    state: tauri::State<'_, AppSecurity>,
) -> Result<(), SecurityError> {
    security_authorize_operation_inner(&password, operation, &state)
}

#[tauri::command]
pub async fn security_reset_local_wallet_data(
    state: tauri::State<'_, AppSecurity>,
) -> Result<(), SecurityError> {
    security_reset_local_wallet_data_inner(&state)
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_local_password_boundary_ready, security_authorize_operation_inner,
        security_get_backend_state_inner, security_get_local_password_policy_inner,
        security_is_unlocked_inner, security_lock_inner, security_probe_backend_inner,
        security_reset_local_wallet_data_inner, security_setup_password_inner,
        security_unlock_inner, AppSecurity, StartupSecurityState,
    };
    use crate::wallet::security::backend::{SecretBackend, SecretBackendAdapter};
    use crate::wallet::security::keystore::Keystore;
    use crate::wallet::security::secret_envelope::{SecretEnvelopeError, StoredSecret};
    use crate::wallet::security::session::SessionManager;
    use crate::wallet::security::types::{
        ForgotPasswordMode, LocalPasswordScope, SecretBackendStatus, SecurityError, SignerOperation,
    };
    use crate::DB;
    use once_cell::sync::Lazy;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Duration;

    static AUTH_STATE_TEST_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    struct DummyKeystore;

    struct UnavailableSecretBackendAdapter;

    impl Keystore for DummyKeystore {
        fn load_mnemonic(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            Ok(None)
        }

        fn load_private_key(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            Ok(None)
        }
    }

    impl SecretBackendAdapter for UnavailableSecretBackendAdapter {
        fn probe(&self) -> Result<(), SecretEnvelopeError> {
            Err(SecretEnvelopeError::Keyring("offline".to_string()))
        }

        fn initialize_empty_store(&self) -> Result<(), SecretEnvelopeError> {
            Err(SecretEnvelopeError::Keyring("offline".to_string()))
        }

        fn encrypt(&self, _plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
            Err(SecretEnvelopeError::Keyring("offline".to_string()))
        }

        fn decrypt(
            &self,
            _secret_data: &str,
            _secret_format: &str,
        ) -> Result<String, SecretEnvelopeError> {
            Err(SecretEnvelopeError::Keyring("offline".to_string()))
        }
    }

    fn reset_security_auth_state() {
        let db = DB.lock().unwrap();
        db.clear_security_password_state_for_tests().unwrap();
        db.clear_local_wallet_data().unwrap();
    }

    #[test]
    fn command_layer_unlock_authorize_lock_sequence() {
        let _guard = AUTH_STATE_TEST_GUARD.lock().unwrap();
        reset_security_auth_state();

        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        assert_eq!(security_is_unlocked_inner(&state), Ok(false));
        assert_eq!(security_setup_password_inner("password"), Ok(()));
        assert_eq!(security_unlock_inner("password", &state), Ok(()));
        assert_eq!(security_is_unlocked_inner(&state), Ok(true));
        assert_eq!(
            state.session_manager().authorize(SignerOperation::Send),
            Err(SecurityError::ReauthRequired)
        );
        assert_eq!(
            security_authorize_operation_inner("password", SignerOperation::Send, &state),
            Ok(())
        );
        assert_eq!(
            state.session_manager().authorize(SignerOperation::Send),
            Ok(())
        );
        assert_eq!(
            state
                .session_manager()
                .authorize(SignerOperation::ExportMnemonic),
            Err(SecurityError::ReauthRequired)
        );
        assert_eq!(security_lock_inner(&state), Ok(()));
        assert_eq!(security_is_unlocked_inner(&state), Ok(false));

        reset_security_auth_state();
    }

    #[test]
    fn wrong_password_does_not_unlock_session() {
        let _guard = AUTH_STATE_TEST_GUARD.lock().unwrap();
        reset_security_auth_state();

        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        let _ = security_setup_password_inner("password");

        assert_eq!(
            security_unlock_inner("wrong", &state),
            Err(SecurityError::WrongPassword)
        );
        assert_eq!(security_is_unlocked_inner(&state), Ok(false));

        reset_security_auth_state();
    }

    #[test]
    fn backend_state_uses_startup_snapshot_and_current_backend_status() {
        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState {
                migration: crate::wallet::security::types::SecretMigrationState {
                    attempted_rows: 2,
                    migrated_rows: 0,
                    skipped_rows: 2,
                    failed_rows: 0,
                },
                has_legacy_plaintext_secrets: true,
            })),
        };

        let backend_state = security_get_backend_state_inner(&state).unwrap();

        assert!(backend_state.degraded);
        assert_eq!(backend_state.migration.attempted_rows, 2);
        assert_eq!(backend_state.backend_status, SecretBackendStatus::Unknown);
    }

    #[test]
    fn unknown_backend_without_legacy_rows_is_not_degraded() {
        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        let backend_state = security_get_backend_state_inner(&state).unwrap();

        assert!(!backend_state.degraded);
        assert_eq!(backend_state.backend_status, SecretBackendStatus::Unknown);
    }

    #[test]
    fn authorize_operation_rejects_wrong_password() {
        let _guard = AUTH_STATE_TEST_GUARD.lock().unwrap();
        reset_security_auth_state();

        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        assert_eq!(security_setup_password_inner("password"), Ok(()));
        assert_eq!(
            security_authorize_operation_inner("wrong", SignerOperation::Send, &state),
            Err(SecurityError::WrongPassword)
        );
    }

    #[test]
    fn local_password_policy_matches_wallet_contract() {
        let policy = security_get_local_password_policy_inner();

        assert_eq!(
            policy.installation_scope,
            LocalPasswordScope::PerInstallation
        );
        assert!(policy.device_only);
        assert!(!policy.cloud_sync);
        assert!(!policy.replaces_recovery_phrase);
        assert!(policy.requires_password_before_create_or_import);
        assert_eq!(policy.idle_lock_seconds, 900);
        assert!(policy.lock_on_system_sleep);
        assert_eq!(
            policy.forgot_password_mode,
            ForgotPasswordMode::ResetLocalDataAndRestoreWithRecoveryMaterial
        );
    }

    #[test]
    fn probe_backend_promotes_unknown_to_ready_when_available() {
        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        let backend_state = security_probe_backend_inner(&state).unwrap();

        assert_ne!(backend_state.backend_status, SecretBackendStatus::Unknown);
    }

    #[test]
    fn reset_local_wallet_data_clears_password_and_locks_session() {
        let _guard = AUTH_STATE_TEST_GUARD.lock().unwrap();
        reset_security_auth_state();

        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        security_setup_password_inner("password").unwrap();
        security_unlock_inner("password", &state).unwrap();
        security_reset_local_wallet_data_inner(&state).unwrap();

        assert_eq!(security_is_unlocked_inner(&state), Ok(false));
        assert_eq!(super::security_has_password_inner(), Ok(false));
    }

    #[test]
    fn wallet_boundary_requires_password_before_create_or_import() {
        let _guard = AUTH_STATE_TEST_GUARD.lock().unwrap();
        reset_security_auth_state();

        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        assert_eq!(
            ensure_local_password_boundary_ready(&state),
            Err(SecurityError::NoPassword)
        );
    }

    #[test]
    fn wallet_boundary_fails_closed_when_backend_unavailable() {
        let _guard = AUTH_STATE_TEST_GUARD.lock().unwrap();
        reset_security_auth_state();

        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(
                Duration::from_secs(30),
                Duration::from_secs(90),
            )),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::with_adapter(Arc::new(
                UnavailableSecretBackendAdapter,
            ))),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        security_setup_password_inner("password").unwrap();

        assert_eq!(
            ensure_local_password_boundary_ready(&state),
            Err(SecurityError::SecretBackendUnavailable)
        );
    }
}
