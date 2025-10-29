use ethers::signers::{LocalWallet, Signer};
use crate::wallet::types::CreateWalletResponse;
use crate::DB;
use std::str::FromStr;

#[tauri::command]
pub fn evm_create_wallet_from_private_key(
    private_key: String,
    wallet_label: Option<String>,
) -> Result<CreateWalletResponse, String> {
    // Parse private key
    let trimmed = private_key.trim();
    
    let wallet: LocalWallet = if trimmed.starts_with("0x") {
        LocalWallet::from_str(trimmed)
            .map_err(|e| format!("Invalid private key: {}", e))?
    } else {
        LocalWallet::from_str(&format!("0x{}", trimmed))
            .map_err(|e| format!("Invalid private key: {}", e))?
    };
    
    let address = wallet.address();
    let address_str = format!("{:?}", address);
    let label = wallet_label.unwrap_or_else(|| "EVM Wallet".to_string());
    
    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet_info = db.add_evm_wallet(
        label,
        "private-key".to_string(),
        address_str,
    ).map_err(|e| format!("Failed to save wallet: {}", e))?;
    
    // Store private key
    db.add_evm_wallet_secret(
        wallet_info.id.clone(),
        private_key.clone(),
        "private-key".to_string(),
    ).map_err(|e| format!("Failed to save private key: {}", e))?;
    
    drop(db);
    
    Ok(CreateWalletResponse {
        mnemonic: private_key,
        wallet: wallet_info,
    })
}

#[tauri::command]
pub fn evm_export_mnemonic(wallet_id: String) -> Result<String, String> {
    let db = DB.lock().unwrap();
    
    // Get wallet to verify it exists
    let wallet = db.get_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))?
        .ok_or_else(|| "Wallet not found".to_string())?;
    
    // Only allow exporting if it's a mnemonic wallet
    if wallet.wallet_type != "mnemonic" {
        return Err("This wallet was imported from a private key, not a mnemonic.".to_string());
    }
    
    // Get the secret
    let (secret_data, secret_type) = db.get_evm_wallet_secret(&wallet_id)
        .map_err(|e| format!("Failed to get wallet secret: {}", e))?
        .ok_or_else(|| "Wallet secret not found".to_string())?;
    
    if secret_type != "mnemonic" {
        return Err("Invalid secret type".to_string());
    }
    
    Ok(secret_data)
}

#[tauri::command]
pub fn evm_export_private_key(wallet_id: String) -> Result<String, String> {
    let db = DB.lock().unwrap();
    
    // Get wallet to verify it exists
    let _wallet = db.get_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))?
        .ok_or_else(|| "Wallet not found".to_string())?;
    
    // Get the secret
    let (secret_data, secret_type) = db.get_evm_wallet_secret(&wallet_id)
        .map_err(|e| format!("Failed to get wallet secret: {}", e))?
        .ok_or_else(|| "Wallet secret not found".to_string())?;
    
    match secret_type.as_str() {
        "private-key" => Ok(secret_data),
        "mnemonic" => {
            // For mnemonic-based wallets, we need to derive the private key from mnemonic
            derive_private_key_from_mnemonic(&secret_data)
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
