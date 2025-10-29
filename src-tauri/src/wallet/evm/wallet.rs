use bip39::{Mnemonic, Language};
use ethers::signers::coins_bip39::English;
use ethers::signers::{MnemonicBuilder, Signer};
use crate::wallet::types::CreateWalletResponse;
use crate::DB;

#[tauri::command]
pub fn evm_create_wallet_from_mnemonic(
    mnemonic_phrase: String,
    wallet_label: Option<String>,
) -> Result<CreateWalletResponse, String> {
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
    
    // Store wallet in database
    let db = DB.lock().unwrap();
    let wallet_info = db.add_evm_wallet(
        label,
        "mnemonic".to_string(),
        address_str,
    ).map_err(|e| format!("Failed to save wallet: {}", e))?;
    
    // Store encrypted mnemonic
    db.add_evm_wallet_secret(
        wallet_info.id.clone(),
        mnemonic_phrase.clone(),
        "mnemonic".to_string(),
    ).map_err(|e| format!("Failed to save mnemonic: {}", e))?;
    
    drop(db);
    
    Ok(CreateWalletResponse {
        mnemonic: mnemonic_phrase,
        wallet: wallet_info,
    })
}
