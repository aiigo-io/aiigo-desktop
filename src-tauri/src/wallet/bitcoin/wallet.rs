use crate::wallet::security::commands::AppSecurity;
use crate::wallet::bitcoin::private_key::map_security_error;
use crate::wallet::security::backend::SecretBackend;
use crate::wallet::security::secret_envelope::StoredSecret;
use crate::wallet::types::CreateWalletResponse;
use crate::DB;
use bip39::{Language, Mnemonic};
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::Network;
use std::str::FromStr;

fn prepare_mnemonic_secret(
    secret_backend: &SecretBackend,
    mnemonic: &str,
) -> Result<StoredSecret, String> {
    secret_backend
        .prepare_encrypted_secret(mnemonic)
        .map_err(map_security_error)
}

#[tauri::command]
pub fn bitcoin_create_wallet_from_mnemonic(
    mnemonic_phrase: String,
    wallet_label: Option<String>,
    reveal_secret: Option<bool>,
    state: tauri::State<'_, AppSecurity>,
) -> Result<CreateWalletResponse, String> {
    // Validate and parse mnemonic
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, &mnemonic_phrase)
        .map_err(|e| format!("Invalid mnemonic: {}", e))?;

    // Generate seed from mnemonic
    let seed = mnemonic.to_seed("");

    // Create master private key from seed
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let master_xprv = Xpriv::new_master(Network::Bitcoin, &seed)
        .map_err(|e| format!("Failed to create master key: {}", e))?;

    // Standard BIP86 derivation path for Taproot: m/86'/0'/0'/0/0
    let derivation_path = DerivationPath::from_str("m/86'/0'/0'/0/0")
        .map_err(|e| format!("Invalid derivation path: {}", e))?;

    let child_xprv = master_xprv
        .derive_priv(&secp, &derivation_path)
        .map_err(|e| format!("Failed to derive child key: {}", e))?;

    let private_key = child_xprv.to_priv();
    let public_key = private_key.public_key(&secp);

    // Convert to XOnlyPublicKey for Taproot (P2TR)
    let xonly_public_key = bitcoin::key::XOnlyPublicKey::from(public_key);

    // Generate Taproot address (bc1p...)
    let address = bitcoin::Address::p2tr(&secp, xonly_public_key, None, Network::Bitcoin);

    let address_str = address.to_string();
    let label = wallet_label.unwrap_or_else(|| "Bitcoin Wallet".to_string());
    let stored_secret = prepare_mnemonic_secret(state.secret_backend(), &mnemonic.to_string())?;

    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet = db
        .insert_bitcoin_wallet_with_secret(
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
            wallet,
            mnemonic.to_string(),
            "mnemonic",
        ))
    } else {
        Ok(CreateWalletResponse::without_revealed_secret(wallet))
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
