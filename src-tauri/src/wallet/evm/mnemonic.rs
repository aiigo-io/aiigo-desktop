use bip39::{Language, Mnemonic};
use rand::Rng;

#[tauri::command]
pub fn evm_create_mnemonic() -> Result<String, String> {
    let mut rng = rand::rng();
    let entropy: [u8; 16] = rng.random();
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;

    Ok(mnemonic.to_string())
}

#[tauri::command]
pub fn evm_import_mnemonic(mnemonic_phrase: String) -> Result<bool, String> {
    Mnemonic::parse_in_normalized(Language::English, &mnemonic_phrase)
        .map_err(|e| format!("Invalid mnemonic: {}", e))?;

    Ok(true)
}
