use crate::wallet::security::keystore::Keystore;
use crate::wallet::security::commands::AppSecurity;
use crate::wallet::security::session::SessionManager;
use crate::wallet::security::types::{SecurityError, SignerOperation};
use crate::wallet::types::CreateWalletResponse;
use crate::DB;
use ethers::signers::{LocalWallet, Signer};
use std::str::FromStr;

pub(crate) fn map_security_error(error: SecurityError) -> String {
    match error {
        SecurityError::Locked => "locked".to_string(),
        SecurityError::Expired => "expired".to_string(),
        SecurityError::PolicyDenied => "policy_denied".to_string(),
        SecurityError::OperationNotAllowed => "operation_not_allowed".to_string(),
        SecurityError::UnknownWallet => "unknown_wallet".to_string(),
        SecurityError::SecretBackendUnavailable => "secret_backend_unavailable".to_string(),
    }
}

fn export_mnemonic_inner(
    wallet_type: &str,
    address: &str,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
) -> Result<String, String> {
    if wallet_type != "mnemonic" {
        return Err("This wallet was imported from a private key, not a mnemonic.".to_string());
    }

    load_authorized_mnemonic(
        address,
        keystore,
        session_manager,
        SignerOperation::ExportMnemonic,
    )
    .map_err(map_security_error)?
    .ok_or_else(|| "Wallet secret not found".to_string())
}

fn export_private_key_inner(
    wallet_type: &str,
    address: &str,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
) -> Result<String, String> {
    match wallet_type {
        "private-key" | "private_key" => load_authorized_private_key(
            address,
            keystore,
            session_manager,
            SignerOperation::ExportPrivateKey,
        )
        .map_err(map_security_error)?
        .ok_or_else(|| "Wallet secret not found".to_string()),
        "mnemonic" => {
            let mnemonic = load_authorized_mnemonic(
                address,
                keystore,
                session_manager,
                SignerOperation::ExportPrivateKey,
            )
            .map_err(map_security_error)?
            .ok_or_else(|| "Wallet secret not found".to_string())?;
            derive_private_key_from_mnemonic(&mnemonic)
        }
        _ => Err("Unknown wallet type".to_string()),
    }
}

pub(crate) fn load_authorized_mnemonic(
    address: &str,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
    operation: SignerOperation,
) -> Result<Option<String>, SecurityError> {
    session_manager.authorize(operation)?;
    keystore.load_mnemonic(address)
}

pub(crate) fn load_authorized_private_key(
    address: &str,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
    operation: SignerOperation,
) -> Result<Option<String>, SecurityError> {
    session_manager.authorize(operation)?;
    keystore.load_private_key(address)
}

#[tauri::command]
pub fn evm_create_wallet_from_private_key(
    private_key: String,
    wallet_label: Option<String>,
    reveal_secret: Option<bool>,
    state: tauri::State<'_, AppSecurity>,
) -> Result<CreateWalletResponse, String> {
    // Parse private key
    let trimmed = private_key.trim();

    let wallet: LocalWallet = if trimmed.starts_with("0x") {
        LocalWallet::from_str(trimmed).map_err(|e| format!("Invalid private key: {}", e))?
    } else {
        LocalWallet::from_str(&format!("0x{}", trimmed))
            .map_err(|e| format!("Invalid private key: {}", e))?
    };

    let address = wallet.address();
    let address_str = format!("{:?}", address);
    let label = wallet_label.unwrap_or_else(|| "EVM Wallet".to_string());
    let stored_secret = state
        .secret_backend()
        .prepare_encrypted_secret(&private_key)
        .map_err(map_security_error)?;

    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet_info = db
        .insert_evm_wallet_with_secret(
            label,
            "private-key".to_string(),
            address_str,
            stored_secret,
            "private-key".to_string(),
        )
        .map_err(|e| format!("Failed to save wallet: {}", e))?;

    drop(db);

    if reveal_secret.unwrap_or(false) {
        Ok(CreateWalletResponse::with_revealed_secret(
            wallet_info,
            private_key,
            "private-key",
        ))
    } else {
        Ok(CreateWalletResponse::without_revealed_secret(wallet_info))
    }
}

#[tauri::command]
pub fn evm_export_mnemonic(
    wallet_id: String,
    state: tauri::State<'_, AppSecurity>,
) -> Result<String, String> {
    let db = DB.lock().unwrap();

    // Get wallet to verify it exists
    let wallet = db
        .get_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))?
        .ok_or_else(|| "Wallet not found".to_string())?;

    let address = wallet.address.clone();
    let wallet_type = wallet.wallet_type.clone();
    drop(db);

    export_mnemonic_inner(&wallet_type, &address, state.keystore(), state.session_manager())
}

#[tauri::command]
pub fn evm_export_private_key(
    wallet_id: String,
    state: tauri::State<'_, AppSecurity>,
) -> Result<String, String> {
    let db = DB.lock().unwrap();

    // Get wallet to verify it exists
    let wallet = db
        .get_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))?
        .ok_or_else(|| "Wallet not found".to_string())?;

    let address = wallet.address.clone();
    let wallet_type = wallet.wallet_type.clone();
    drop(db);

    export_private_key_inner(&wallet_type, &address, state.keystore(), state.session_manager())
}

fn derive_private_key_from_mnemonic(mnemonic_str: &str) -> Result<String, String> {
    use ethers::signers::coins_bip39::English;
    use ethers::signers::MnemonicBuilder;

    let builder = MnemonicBuilder::<English>::default()
        .phrase(mnemonic_str)
        .derivation_path("m/44'/60'/0'/0/0")
        .map_err(|e| format!("Failed to set derivation path: {}", e))?;

    let wallet = builder
        .build()
        .map_err(|e| format!("Failed to build wallet: {}", e))?;

    // Get the signing key and encode as hex
    let key_bytes = wallet.signer().to_bytes();
    Ok(format!("0x{}", hex::encode(key_bytes)))
}

#[cfg(test)]
mod tests {
    use super::{export_mnemonic_inner, load_authorized_mnemonic, load_authorized_private_key};
    use crate::wallet::security::keystore::Keystore;
    use crate::wallet::security::session::SessionManager;
    use crate::wallet::security::types::{SecurityError, SignerOperation};
    use std::time::Duration;

    struct StubKeystore;

    struct PanicKeystore;

    impl Keystore for StubKeystore {
        fn load_mnemonic(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            Ok(Some("seed words".to_string()))
        }

        fn load_private_key(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            Ok(Some("0xdeadbeef".to_string()))
        }
    }

    impl Keystore for PanicKeystore {
        fn load_mnemonic(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            panic!("keystore should not be called while session is locked");
        }

        fn load_private_key(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            panic!("keystore should not be called while session is locked");
        }
    }

    #[test]
    fn export_mnemonic_returns_locked_without_keystore_access() {
        let session = SessionManager::new(Duration::from_secs(30));

        assert_eq!(
            load_authorized_mnemonic(
                "0x1234",
                &PanicKeystore,
                &session,
                SignerOperation::ExportMnemonic
            ),
            Err(SecurityError::Locked)
        );
    }

    #[test]
    fn export_private_key_returns_locked_without_keystore_access() {
        let session = SessionManager::new(Duration::from_secs(30));

        assert_eq!(
            load_authorized_private_key(
                "0x1234",
                &PanicKeystore,
                &session,
                SignerOperation::ExportPrivateKey
            ),
            Err(SecurityError::Locked)
        );
    }

    #[test]
    fn load_authorized_mnemonic_returns_secret_for_send_when_session_unlocked() {
        let session = SessionManager::new(Duration::from_secs(30));
        session.unlock("token").unwrap();

        assert_eq!(
            load_authorized_mnemonic("0x1234", &StubKeystore, &session, SignerOperation::Send),
            Ok(Some("seed words".to_string()))
        );
    }

    #[test]
    fn export_mnemonic_returns_policy_denied_when_session_unlocked() {
        let session = SessionManager::new(Duration::from_secs(30));
        session.unlock("token").unwrap();

        assert_eq!(
            export_mnemonic_inner("mnemonic", "0x1234", &PanicKeystore, &session),
            Err("policy_denied".to_string())
        );
    }
}
