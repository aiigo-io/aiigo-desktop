use crate::wallet::security::keystore::Keystore;
use crate::wallet::security::commands::AppSecurity;
use crate::wallet::security::session::SessionManager;
use crate::wallet::security::types::{SecurityError, SignerOperation};
use crate::wallet::types::CreateWalletResponse;
use crate::DB;
use bitcoin::{Address, Network};
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

/// Validates a private key string (WIF or hex format)
fn validate_private_key(
    key_str: &str,
) -> Result<(bitcoin::secp256k1::SecretKey, bitcoin::key::PublicKey), String> {
    let trimmed = key_str.trim();

    // Try WIF format first
    if let Ok(private_key) = bitcoin::PrivateKey::from_str(trimmed) {
        let secret = private_key.inner;
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let public_key = bitcoin::key::PublicKey::new(
            bitcoin::secp256k1::Keypair::from_secret_key(&secp, &secret).public_key(),
        );
        return Ok((secret, public_key));
    }

    // Try hex format (64 hex chars = 32 bytes)
    if trimmed.len() == 64 {
        if let Ok(bytes) = hex::decode(trimmed) {
            if bytes.len() == 32 {
                if let Ok(secret_key) = bitcoin::secp256k1::SecretKey::from_slice(&bytes) {
                    let secp = bitcoin::secp256k1::Secp256k1::new();
                    let public_key = bitcoin::key::PublicKey::new(
                        bitcoin::secp256k1::Keypair::from_secret_key(&secp, &secret_key)
                            .public_key(),
                    );
                    return Ok((secret_key, public_key));
                }
            }
        }
    }

    Err(
        "Invalid private key format. Please provide a valid WIF or 64-character hex string."
            .to_string(),
    )
}

#[tauri::command]
pub fn bitcoin_create_wallet_from_private_key(
    private_key: String,
    wallet_label: Option<String>,
) -> Result<CreateWalletResponse, String> {
    // Validate and parse private key
    let (_secret, public_key) = validate_private_key(&private_key)?;

    let secp = bitcoin::secp256k1::Secp256k1::new();

    // Convert to XOnlyPublicKey for Taproot (P2TR)
    let xonly_public_key = bitcoin::key::XOnlyPublicKey::from(public_key);

    // Generate Taproot address (bc1p...)
    let address = Address::p2tr(&secp, xonly_public_key, None, Network::Bitcoin);
    let address_str = address.to_string();
    let label = wallet_label.unwrap_or_else(|| "Bitcoin Wallet".to_string());

    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet = db
        .add_bitcoin_wallet(label, "private-key".to_string(), address_str)
        .map_err(|e| format!("Failed to save wallet: {}", e))?;

    // Secret writes remain DB-backed in the current MVP; keystore writes are deferred.
    // Store private key (for now, storing as plain text - TODO: add encryption)
    db.add_wallet_secret(
        wallet.id.clone(),
        private_key.clone(),
        "private-key".to_string(),
    )
    .map_err(|e| format!("Failed to save private key: {}", e))?;

    drop(db);

    Ok(CreateWalletResponse {
        mnemonic: private_key, // Return the private key as "mnemonic" for compatibility
        wallet,
    })
}

#[tauri::command]
pub fn bitcoin_export_mnemonic(
    wallet_id: String,
    state: tauri::State<'_, AppSecurity>,
) -> Result<String, String> {
    let db = DB.lock().unwrap();

    // Get wallet to verify it exists
    let wallet = db
        .get_bitcoin_wallet(&wallet_id)
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
pub fn bitcoin_export_private_key(
    wallet_id: String,
    state: tauri::State<'_, AppSecurity>,
) -> Result<String, String> {
    let db = DB.lock().unwrap();

    // Get wallet to verify it exists
    let wallet = db
        .get_bitcoin_wallet(&wallet_id)
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

/// Derive the private key from a mnemonic phrase
fn derive_private_key_from_mnemonic(mnemonic_str: &str) -> Result<String, String> {
    use bip39::{Language, Mnemonic};
    use bitcoin::bip32::{DerivationPath, Xpriv};
    use std::str::FromStr;

    // Parse mnemonic
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, mnemonic_str)
        .map_err(|e| format!("Invalid mnemonic: {}", e))?;

    // Generate seed
    let seed = mnemonic.to_seed("");

    // Create master private key
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let master_xprv = Xpriv::new_master(Network::Bitcoin, &seed)
        .map_err(|e| format!("Failed to create master key: {}", e))?;

    // Standard BIP44 derivation path: m/44'/0'/0'/0/0
    let derivation_path = DerivationPath::from_str("m/44'/0'/0'/0/0")
        .map_err(|e| format!("Invalid derivation path: {}", e))?;

    let child_xprv = master_xprv
        .derive_priv(&secp, &derivation_path)
        .map_err(|e| format!("Failed to derive child key: {}", e))?;

    let private_key = child_xprv.to_priv();

    // Return as WIF format
    Ok(private_key.to_string())
}

#[cfg(test)]
mod tests {
    use super::{load_authorized_mnemonic, load_authorized_private_key};
    use crate::wallet::security::keystore::Keystore;
    use crate::wallet::security::session::SessionManager;
    use crate::wallet::security::types::{SecurityError, SignerOperation};
    use std::time::Duration;

    struct PanicKeystore;

    struct StubKeystore;

    impl Keystore for PanicKeystore {
        fn load_mnemonic(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            panic!("keystore should not be called while session is locked");
        }

        fn load_private_key(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            panic!("keystore should not be called while session is locked");
        }
    }

    impl Keystore for StubKeystore {
        fn load_mnemonic(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            Ok(Some("seed words".to_string()))
        }

        fn load_private_key(&self, _address: &str) -> Result<Option<String>, SecurityError> {
            Ok(Some("private-key".to_string()))
        }
    }

    #[test]
    fn export_mnemonic_returns_locked_without_keystore_access() {
        let session = SessionManager::new(Duration::from_secs(30));

        assert_eq!(
            load_authorized_mnemonic(
                "bc1ptestaddress",
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
                "bc1ptestaddress",
                &PanicKeystore,
                &session,
                SignerOperation::ExportPrivateKey
            ),
            Err(SecurityError::Locked)
        );
    }

    #[test]
    fn load_authorized_mnemonic_returns_secret_when_session_unlocked() {
        let session = SessionManager::new(Duration::from_secs(30));
        session.unlock("token").unwrap();

        assert_eq!(
            load_authorized_mnemonic(
                "bc1ptestaddress",
                &StubKeystore,
                &session,
                SignerOperation::ExportMnemonic
            ),
            Ok(Some("seed words".to_string()))
        );
    }
}
