use super::keystore::Keystore;
use super::session::SessionManager;
use super::types::SecurityError;
use std::sync::Arc;

pub struct AppSecurity {
    pub session_manager: Arc<SessionManager>,
    pub keystore: Arc<dyn Keystore + Send + Sync>,
}

impl AppSecurity {
    pub fn session_manager(&self) -> &SessionManager {
        self.session_manager.as_ref()
    }

    pub fn keystore(&self) -> &(dyn Keystore + Send + Sync) {
        self.keystore.as_ref()
    }
}

fn security_unlock_inner(token: &str, state: &AppSecurity) -> Result<(), SecurityError> {
    state.session_manager().unlock(token)
}

fn security_lock_inner(state: &AppSecurity) -> Result<(), SecurityError> {
    state.session_manager().lock();
    Ok(())
}

fn security_is_unlocked_inner(state: &AppSecurity) -> Result<bool, SecurityError> {
    Ok(state.session_manager().is_unlocked())
}

#[tauri::command]
pub async fn security_unlock(
    token: String,
    state: tauri::State<'_, AppSecurity>,
) -> Result<(), SecurityError> {
    security_unlock_inner(&token, &state)
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

#[cfg(test)]
mod tests {
    use super::{
        security_is_unlocked_inner, security_lock_inner, security_unlock_inner, AppSecurity,
    };
    use crate::wallet::security::keystore::Keystore;
    use crate::wallet::security::session::SessionManager;
    use crate::wallet::security::types::{SecurityError, SignerOperation};
    use std::sync::Arc;
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
        };

        assert_eq!(security_is_unlocked_inner(&state), Ok(false));
        assert_eq!(security_unlock_inner("token", &state), Ok(()));
        assert_eq!(security_is_unlocked_inner(&state), Ok(true));
        assert_eq!(
            state.session_manager().authorize(SignerOperation::Send),
            Ok(())
        );
        assert_eq!(security_lock_inner(&state), Ok(()));
        assert_eq!(security_is_unlocked_inner(&state), Ok(false));
    }
}
