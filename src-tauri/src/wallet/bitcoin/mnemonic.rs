use bip39::{Mnemonic, Language};
use rand::RngCore;


#[tauri::command]
pub fn bitcoin_create_mnemonic() -> String {
  // 生成 128-bit 随机熵（对应 12 个助记词）
  let mut entropy = [0u8; 16];
  rand::rng().fill_bytes(&mut entropy);

  // 从熵生成助记词
  let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy).unwrap();
  mnemonic.to_string()
}

#[tauri::command]
pub fn bitcoin_import_mnemonic(phrase: String) -> Result<String, String> {
  match Mnemonic::parse_in_normalized(Language::English, &phrase) {
      Ok(m) => Ok(m.to_string()),
      Err(e) => Err(format!("Invalid mnemonic: {}", e)),
  }
}
