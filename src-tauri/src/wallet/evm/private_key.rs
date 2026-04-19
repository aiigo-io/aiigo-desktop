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
        SecurityError::PolicyDenied => "policy_denied".to_string(),
        SecurityError::OperationNotAllowed => "operation_not_allowed".to_string(),
        SecurityError::UnknownWallet => "unknown_wallet".to_string(),
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

    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet_info = db
        .add_evm_wallet(label, "private-key".to_string(), address_str)
        .map_err(|e| format!("Failed to save wallet: {}", e))?;

    // Secret writes remain DB-backed in Phase 1; Keystore write API is deferred.
    // Store private key
    db.add_evm_wallet_secret(
        wallet_info.id.clone(),
        private_key.clone(),
        "private-key".to_string(),
    )
    .map_err(|e| format!("Failed to save private key: {}", e))?;

    drop(db);

    Ok(CreateWalletResponse {
        mnemonic: private_key,
        wallet: wallet_info,
    })
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

    // Only allow exporting if it's a mnemonic wallet
    if wallet.wallet_type != "mnemonic" {
        return Err("This wallet was imported from a private key, not a mnemonic.".to_string());
    }

    let address = wallet.address.clone();
    drop(db);

    load_authorized_mnemonic(
        &address,
        state.keystore(),
        state.session_manager(),
        SignerOperation::ExportMnemonic,
    )
    .map_err(map_security_error)?
    .ok_or_else(|| "Wallet secret not found".to_string())
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

    match wallet_type.as_str() {
        "private-key" | "private_key" => load_authorized_private_key(
            &address,
            state.keystore(),
            state.session_manager(),
            SignerOperation::ExportPrivateKey,
        )
        .map_err(map_security_error)?
        .ok_or_else(|| "Wallet secret not found".to_string()),
        "mnemonic" => {
            // For mnemonic-based wallets, we need to derive the private key from mnemonic
            let mnemonic = load_authorized_mnemonic(
                &address,
                state.keystore(),
                state.session_manager(),
                SignerOperation::ExportPrivateKey,
            )
            .map_err(map_security_error)?
            .ok_or_else(|| "Wallet secret not found".to_string())?;
            derive_private_key_from_mnemonic(&mnemonic)
        }
        _ => Err("Unknown wallet type".to_string()),
    }
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
    use super::{load_authorized_mnemonic, load_authorized_private_key};
    use crate::wallet::security::keystore::Keystore;
    use crate::wallet::security::session::SessionManager;
    use crate::wallet::security::types::{SecurityError, SignerOperation};
    use std::time::Duration;

    struct PanicKeystore;

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
}
