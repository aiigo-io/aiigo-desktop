use crate::wallet::security::backend::SecretBackend;
use crate::wallet::security::commands::ensure_local_password_configured;
use crate::wallet::security::commands::AppSecurity;
use crate::wallet::security::keystore::Keystore;
use crate::wallet::security::session::SessionManager;
use crate::wallet::security::types::{SecurityError, SignerOperation};
use crate::wallet::types::CreateWalletResponse;
use crate::DB;
use bitcoin::{Address, Network};
use std::str::FromStr;

pub(crate) fn map_security_error(error: SecurityError) -> String {
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

fn export_mnemonic_inner(
    wallet_type: &str,
    address: &str,
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
) -> Result<String, String> {
    if wallet_type != "mnemonic" {
        return Err("This wallet was imported from a private key, not a mnemonic.".to_string());
    }

    load_authorized_mnemonic(
        address,
        secret_backend,
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
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
) -> Result<String, String> {
    match wallet_type {
        "private-key" | "private_key" => load_authorized_private_key(
            address,
            secret_backend,
            keystore,
            session_manager,
            SignerOperation::ExportPrivateKey,
        )
        .map_err(map_security_error)?
        .ok_or_else(|| "Wallet secret not found".to_string()),
        "mnemonic" => {
            let mnemonic = load_authorized_mnemonic(
                address,
                secret_backend,
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
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
    operation: SignerOperation,
) -> Result<Option<String>, SecurityError> {
    session_manager.authorize(operation)?;
    secret_backend.ensure_ready_for_command()?;
    keystore.load_mnemonic(address)
}

pub(crate) fn load_authorized_private_key(
    address: &str,
    secret_backend: &SecretBackend,
    keystore: &(dyn Keystore + Send + Sync),
    session_manager: &SessionManager,
    operation: SignerOperation,
) -> Result<Option<String>, SecurityError> {
    session_manager.authorize(operation)?;
    secret_backend.ensure_ready_for_command()?;
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
    reveal_secret: Option<bool>,
    state: tauri::State<'_, AppSecurity>,
) -> Result<CreateWalletResponse, String> {
    ensure_local_password_configured().map_err(map_security_error)?;

    // Validate and parse private key
    let (_secret, public_key) = validate_private_key(&private_key)?;

    let secp = bitcoin::secp256k1::Secp256k1::new();

    // Convert to XOnlyPublicKey for Taproot (P2TR)
    let xonly_public_key = bitcoin::key::XOnlyPublicKey::from(public_key);

    // Generate Taproot address (bc1p...)
    let address = Address::p2tr(&secp, xonly_public_key, None, Network::Bitcoin);
    let address_str = address.to_string();
    let label = wallet_label.unwrap_or_else(|| "Bitcoin Wallet".to_string());
    let has_existing_secrets = {
        let db = DB.lock().unwrap();
        db.has_any_wallet_secret_rows()
            .map_err(|e| format!("Failed to inspect existing wallet secrets: {}", e))?
    };
    if !has_existing_secrets {
        state
            .secret_backend()
            .initialize_for_empty_store()
            .map_err(map_security_error)?;
    }
    let stored_secret = state
        .secret_backend()
        .prepare_encrypted_secret(&private_key)
        .map_err(map_security_error)?;

    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet = db
        .insert_bitcoin_wallet_with_secret(
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
            wallet,
            private_key,
            "private-key",
        ))
    } else {
        Ok(CreateWalletResponse::without_revealed_secret(wallet))
    }
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

    let address = wallet.address.clone();
    let wallet_type = wallet.wallet_type.clone();
    drop(db);

    export_mnemonic_inner(
        &wallet_type,
        &address,
        state.secret_backend(),
        state.keystore(),
        state.session_manager(),
    )
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

    export_private_key_inner(
        &wallet_type,
        &address,
        state.secret_backend(),
        state.keystore(),
        state.session_manager(),
    )
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
    use super::{
        export_mnemonic_inner, export_private_key_inner, load_authorized_mnemonic,
        load_authorized_private_key,
    };
    use crate::wallet::security::backend::{SecretBackend, SecretBackendAdapter};
    use crate::wallet::security::keystore::Keystore;
    use crate::wallet::security::secret_envelope::{SecretEnvelopeError, StoredSecret};
    use crate::wallet::security::session::SessionManager;
    use crate::wallet::security::types::{SecurityError, SignerOperation};
    use std::sync::Arc;
    use std::time::Duration;

    struct PanicKeystore;

    struct StubKeystore;

    struct ReadySecretBackendAdapter;

    struct UnavailableSecretBackendAdapter;

    impl SecretBackendAdapter for ReadySecretBackendAdapter {
        fn probe(&self) -> Result<(), SecretEnvelopeError> {
            Ok(())
        }

        fn initialize_empty_store(&self) -> Result<(), SecretEnvelopeError> {
            Ok(())
        }

        fn encrypt(&self, _plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
            unreachable!()
        }

        fn decrypt(
            &self,
            _secret_data: &str,
            _secret_format: &str,
        ) -> Result<String, SecretEnvelopeError> {
            unreachable!()
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
            unreachable!()
        }

        fn decrypt(
            &self,
            _secret_data: &str,
            _secret_format: &str,
        ) -> Result<String, SecretEnvelopeError> {
            unreachable!()
        }
    }

    fn ready_backend() -> SecretBackend {
        SecretBackend::with_adapter(Arc::new(ReadySecretBackendAdapter))
    }

    fn unavailable_backend() -> SecretBackend {
        SecretBackend::with_adapter(Arc::new(UnavailableSecretBackendAdapter))
    }

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
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let backend = ready_backend();

        assert_eq!(
            load_authorized_mnemonic(
                "bc1ptestaddress",
                &backend,
                &PanicKeystore,
                &session,
                SignerOperation::ExportMnemonic
            ),
            Err(SecurityError::Locked)
        );
    }

    #[test]
    fn export_private_key_returns_locked_without_keystore_access() {
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let backend = ready_backend();

        assert_eq!(
            load_authorized_private_key(
                "bc1ptestaddress",
                &backend,
                &PanicKeystore,
                &session,
                SignerOperation::ExportPrivateKey
            ),
            Err(SecurityError::Locked)
        );
    }

    #[test]
    fn load_authorized_mnemonic_returns_secret_after_send_reauth() {
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let backend = ready_backend();
        session
            .authorize_verified_operation(SignerOperation::Send)
            .unwrap();

        assert_eq!(
            load_authorized_mnemonic(
                "bc1ptestaddress",
                &backend,
                &StubKeystore,
                &session,
                SignerOperation::Send,
            ),
            Ok(Some("seed words".to_string()))
        );
    }

    #[test]
    fn send_returns_secret_backend_unavailable_without_keystore_access() {
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let backend = unavailable_backend();
        session
            .authorize_verified_operation(SignerOperation::Send)
            .unwrap();

        assert_eq!(
            load_authorized_mnemonic(
                "bc1ptestaddress",
                &backend,
                &PanicKeystore,
                &session,
                SignerOperation::Send,
            ),
            Err(SecurityError::SecretBackendUnavailable)
        );
    }

    #[test]
    fn export_mnemonic_requires_fresh_reauth_when_only_unlocked() {
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let backend = ready_backend();
        session.unlock_verified().unwrap();

        assert_eq!(
            export_mnemonic_inner(
                "mnemonic",
                "bc1ptestaddress",
                &backend,
                &PanicKeystore,
                &session,
            ),
            Err("reauth_required".to_string())
        );
    }

    #[test]
    fn export_mnemonic_returns_expired_after_ttl() {
        let session = SessionManager::new(Duration::from_millis(1), Duration::from_secs(90));
        let backend = ready_backend();
        session
            .authorize_verified_operation(SignerOperation::ExportMnemonic)
            .unwrap();
        std::thread::sleep(Duration::from_millis(5));

        assert_eq!(
            export_mnemonic_inner(
                "mnemonic",
                "bc1ptestaddress",
                &backend,
                &PanicKeystore,
                &session,
            ),
            Err("expired".to_string())
        );
    }

    #[test]
    fn export_private_key_requires_fresh_reauth_when_only_unlocked() {
        let session = SessionManager::new(Duration::from_secs(30), Duration::from_secs(90));
        let backend = ready_backend();
        session.unlock_verified().unwrap();

        assert_eq!(
            export_private_key_inner(
                "private-key",
                "bc1ptestaddress",
                &backend,
                &PanicKeystore,
                &session,
            ),
            Err("reauth_required".to_string())
        );
    }
}
