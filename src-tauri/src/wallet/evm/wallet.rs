use crate::wallet::evm::private_key::map_security_error;
use crate::wallet::security::backend::SecretBackend;
use crate::wallet::security::commands::ensure_local_password_configured;
use crate::wallet::security::commands::AppSecurity;
use crate::wallet::security::secret_envelope::StoredSecret;
use crate::wallet::types::CreateWalletResponse;
use crate::DB;
use bip39::{Language, Mnemonic};
use ethers::signers::coins_bip39::English;
use ethers::signers::{MnemonicBuilder, Signer};

fn prepare_mnemonic_secret(
    secret_backend: &SecretBackend,
    mnemonic_phrase: &str,
) -> Result<StoredSecret, String> {
    secret_backend
        .prepare_encrypted_secret(mnemonic_phrase)
        .map_err(map_security_error)
}

#[tauri::command]
pub fn evm_create_wallet_from_mnemonic(
    mnemonic_phrase: String,
    wallet_label: Option<String>,
    reveal_secret: Option<bool>,
    state: tauri::State<'_, AppSecurity>,
) -> Result<CreateWalletResponse, String> {
    ensure_local_password_configured().map_err(map_security_error)?;

    // Validate mnemonic
    let _mnemonic = Mnemonic::parse_in_normalized(Language::English, &mnemonic_phrase)
        .map_err(|e| format!("Invalid mnemonic: {}", e))?;

    // Create wallet from mnemonic using ethers-rs
    let builder = MnemonicBuilder::<English>::default()
        .phrase(mnemonic_phrase.as_str())
        .derivation_path("m/44'/60'/0'/0/0")
        .map_err(|e| format!("Failed to set derivation path: {}", e))?;

    let wallet = builder
        .build()
        .map_err(|e| format!("Failed to build wallet: {}", e))?;

    let address = wallet.address();
    let address_str = format!("{:?}", address);
    let label = wallet_label.unwrap_or_else(|| "EVM Wallet".to_string());
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
    let stored_secret = prepare_mnemonic_secret(state.secret_backend(), &mnemonic_phrase)?;

    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet_info = db
        .insert_evm_wallet_with_secret(
            label,
            "mnemonic".to_string(),
            address_str,
            stored_secret,
            "mnemonic".to_string(),
        )
        .map_err(|e| format!("Failed to save wallet: {}", e))?;

    drop(db);

    if reveal_secret.unwrap_or(false) {
        Ok(CreateWalletResponse::with_revealed_secret(
            wallet_info,
            mnemonic_phrase,
            "mnemonic",
        ))
    } else {
        Ok(CreateWalletResponse::without_revealed_secret(wallet_info))
    }
}

#[cfg(test)]
mod tests {
    use super::prepare_mnemonic_secret;
    use crate::wallet::security::backend::{SecretBackend, SecretBackendAdapter};
    use crate::wallet::security::secret_envelope::{SecretEnvelopeError, StoredSecret};
    use std::sync::Arc;

    struct UnavailableSecretBackendAdapter;

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

    #[test]
    fn mnemonic_create_maps_backend_unavailable_to_machine_readable_error() {
        let secret_backend = SecretBackend::with_adapter(Arc::new(UnavailableSecretBackendAdapter));

        assert_eq!(
            prepare_mnemonic_secret(&secret_backend, "seed words"),
            Err("secret_backend_unavailable".to_string())
        );
    }
}
