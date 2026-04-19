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

#[tauri::command]
pub async fn security_unlock(
    token: String,
    state: tauri::State<'_, AppSecurity>,
) -> Result<(), SecurityError> {
    state.session_manager().unlock(&token)
}

#[tauri::command]
pub async fn security_lock(
    state: tauri::State<'_, AppSecurity>,
) -> Result<(), SecurityError> {
    state.session_manager().lock();
    Ok(())
}

#[tauri::command]
pub async fn security_is_unlocked(
    state: tauri::State<'_, AppSecurity>,
) -> Result<bool, SecurityError> {
    Ok(state.session_manager().is_unlocked())
}
