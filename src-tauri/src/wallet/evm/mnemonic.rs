use crate::wallet::security::commands::{ensure_local_password_configured, AppSecurity};
use crate::wallet::security::types::SecurityError;
use bip39::{Language, Mnemonic};
use rand::Rng;

fn map_security_error(error: SecurityError) -> String {
    match error {
        SecurityError::Locked => "locked".to_string(),
        SecurityError::Expired => "expired".to_string(),
        SecurityError::NoPassword => "no_password".to_string(),
        SecurityError::WrongPassword => "wrong_password".to_string(),
        SecurityError::ReauthRequired => "reauth_required".to_string(),
        SecurityError::PolicyDenied => "policy_denied".to_string(),
        SecurityError::OperationNotAllowed => "operation_not_allowed".to_string(),
        SecurityError::UnknownWallet => "unknown_wallet".to_string(),
        SecurityError::SecretBackendUnavailable => "secret_backend_unavailable".to_string(),
    }
}

fn evm_create_mnemonic_inner(state: &AppSecurity) -> Result<String, String> {
    let _ = state;
    ensure_local_password_configured().map_err(map_security_error)?;

    let mut rng = rand::rng();
    let entropy: [u8; 16] = rng.random();
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;

    Ok(mnemonic.to_string())
}

#[tauri::command]
pub fn evm_create_mnemonic(state: tauri::State<'_, AppSecurity>) -> Result<String, String> {
    evm_create_mnemonic_inner(&state)
}

#[tauri::command]
pub fn evm_import_mnemonic(mnemonic_phrase: String) -> Result<bool, String> {
    Mnemonic::parse_in_normalized(Language::English, &mnemonic_phrase)
        .map_err(|e| format!("Invalid mnemonic: {}", e))?;

    Ok(true)
}
