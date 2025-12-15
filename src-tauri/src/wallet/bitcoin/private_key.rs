use crate::wallet::types::CreateWalletResponse;
use crate::DB;
use bitcoin::{Address, Network};
use std::str::FromStr;

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
pub fn bitcoin_export_mnemonic(wallet_id: String) -> Result<String, String> {
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

    // Get the secret
    let (secret_data, secret_type) = db
        .get_wallet_secret(&wallet_id)
        .map_err(|e| format!("Failed to get wallet secret: {}", e))?
        .ok_or_else(|| "Wallet secret not found".to_string())?;

    if secret_type != "mnemonic" {
        return Err("Invalid secret type".to_string());
    }

    Ok(secret_data)
}

#[tauri::command]
pub fn bitcoin_export_private_key(wallet_id: String) -> Result<String, String> {
    let db = DB.lock().unwrap();

    // Get wallet to verify it exists
    let wallet = db
        .get_bitcoin_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))?
        .ok_or_else(|| "Wallet not found".to_string())?;

    // Get the secret
    let (secret_data, _) = db
        .get_wallet_secret(&wallet_id)
        .map_err(|e| format!("Failed to get wallet secret: {}", e))?
        .ok_or_else(|| "Wallet secret not found".to_string())?;

    match wallet.wallet_type.as_str() {
        "private-key" => Ok(secret_data),
        "mnemonic" => {
            // For mnemonic-based wallets, we need to derive the private key from mnemonic
            derive_private_key_from_mnemonic(&secret_data)
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
