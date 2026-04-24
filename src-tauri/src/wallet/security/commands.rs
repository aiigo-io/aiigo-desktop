use super::auth::{hash_password, verify_password};
use super::backend::SecretBackend;
use super::keystore::Keystore;
use super::session::SessionManager;
use super::types::{SecretMigrationState, SecurityBackendState, SecurityError};
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
    let db = DB.lock().map_err(|_| SecurityError::OperationNotAllowed)?;
    let auth_state = db
        .load_security_password_state()
        .map_err(|_| SecurityError::OperationNotAllowed)?
        .ok_or(SecurityError::NoPassword)?;
    drop(db);

    if !verify_password(password, &auth_state)? {
        state.session_manager().lock();
        return Err(SecurityError::WrongPassword);
    }

    state.session_manager().unlock_verified()
}

fn security_lock_inner(state: &AppSecurity) -> Result<(), SecurityError> {
    state.session_manager().lock();
    Ok(())
}

fn security_is_unlocked_inner(state: &AppSecurity) -> Result<bool, SecurityError> {
    Ok(state.session_manager().is_unlocked())
}

fn security_get_backend_state_inner(state: &AppSecurity) -> Result<SecurityBackendState, SecurityError> {
    let startup_state = state
        .startup_state()
        .lock()
        .map_err(|_| SecurityError::OperationNotAllowed)?
        .clone();
    let backend_status = state.secret_backend().current_status();
    let degraded = matches!(backend_status, super::types::SecretBackendStatus::Unavailable { .. })
        || startup_state.has_legacy_plaintext_secrets
        || startup_state.migration.failed_rows > 0;

    Ok(SecurityBackendState {
        backend_status,
        migration: startup_state.migration,
        has_legacy_plaintext_secrets: startup_state.has_legacy_plaintext_secrets,
        degraded,
    })
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
pub async fn security_lock(
    state: tauri::State<'_, AppSecurity>,
) -> Result<(), SecurityError> {
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

#[cfg(test)]
mod tests {
    use super::{
        security_get_backend_state_inner, security_is_unlocked_inner, security_lock_inner,
        security_setup_password_inner, security_unlock_inner, AppSecurity, StartupSecurityState,
    };
    use crate::wallet::security::backend::SecretBackend;
    use crate::wallet::security::keystore::Keystore;
    use crate::wallet::security::session::SessionManager;
    use crate::wallet::security::types::{SecretBackendStatus, SecurityError, SignerOperation};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Duration;

    struct DummyKeystore;

    impl Keystore for DummyKeystore {
        fn load_mnemonic(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            Ok(None)
        }

        fn load_private_key(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            Ok(None)
        }
    }

    #[test]
    fn command_layer_unlock_authorize_lock_sequence() {
        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(Duration::from_secs(30))),
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
            Ok(())
        );
        assert_eq!(
            state.session_manager().authorize(SignerOperation::ExportMnemonic),
            Err(SecurityError::PolicyDenied)
        );
        assert_eq!(security_lock_inner(&state), Ok(()));
        assert_eq!(security_is_unlocked_inner(&state), Ok(false));
    }

    #[test]
    fn wrong_password_does_not_unlock_session() {
        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(Duration::from_secs(30))),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        let _ = security_setup_password_inner("password");

        assert_eq!(security_unlock_inner("wrong", &state), Err(SecurityError::WrongPassword));
        assert_eq!(security_is_unlocked_inner(&state), Ok(false));
    }

    #[test]
    fn backend_state_uses_startup_snapshot_and_current_backend_status() {
        let state = AppSecurity {
            session_manager: Arc::new(SessionManager::new(Duration::from_secs(30))),
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
            session_manager: Arc::new(SessionManager::new(Duration::from_secs(30))),
            keystore: Arc::new(DummyKeystore),
            secret_backend: Arc::new(SecretBackend::new()),
            startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
        };

        let backend_state = security_get_backend_state_inner(&state).unwrap();

        assert!(!backend_state.degraded);
        assert_eq!(backend_state.backend_status, SecretBackendStatus::Unknown);
    }
}
