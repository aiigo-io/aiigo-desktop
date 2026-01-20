use crate::wallet::types::CreateWalletResponse;
use crate::DB;
use bip39::{Language, Mnemonic};
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::Network;
use std::str::FromStr;

#[tauri::command]
pub fn bitcoin_create_wallet_from_mnemonic(
    mnemonic_phrase: String,
    wallet_label: Option<String>,
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

    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet = db
        .add_bitcoin_wallet(label, "mnemonic".to_string(), address_str)
        .map_err(|e| format!("Failed to save wallet: {}", e))?;

    // Store encrypted mnemonic (for now, storing as plain text - TODO: add encryption)
    db.add_wallet_secret(
        wallet.id.clone(),
        mnemonic.to_string(),
        "mnemonic".to_string(),
    )
    .map_err(|e| format!("Failed to save mnemonic: {}", e))?;

    drop(db);

    Ok(CreateWalletResponse {
        mnemonic: mnemonic.to_string(),
        wallet,
    })
}
