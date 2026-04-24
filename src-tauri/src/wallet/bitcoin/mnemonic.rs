use crate::wallet::security::commands::{ensure_local_password_boundary_ready, AppSecurity};
use crate::wallet::security::types::SecurityError;
use bip39::{Language, Mnemonic};
use rand::RngCore;

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

fn bitcoin_create_mnemonic_inner(state: &AppSecurity) -> Result<String, String> {
    ensure_local_password_boundary_ready(state).map_err(map_security_error)?;

    // 生成 128-bit 随机熵（对应 12 个助记词）
    let mut entropy = [0u8; 16];
    rand::rng().fill_bytes(&mut entropy);

    // 从熵生成助记词
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy).unwrap();
    Ok(mnemonic.to_string())
}

#[tauri::command]
pub fn bitcoin_create_mnemonic(state: tauri::State<'_, AppSecurity>) -> Result<String, String> {
    bitcoin_create_mnemonic_inner(&state)
}

#[tauri::command]
pub fn bitcoin_import_mnemonic(phrase: String) -> Result<String, String> {
    match Mnemonic::parse_in_normalized(Language::English, &phrase) {
        Ok(m) => Ok(m.to_string()),
        Err(e) => Err(format!("Invalid mnemonic: {}", e)),
    }
}
