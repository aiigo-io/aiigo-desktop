mod db;
mod wallet;

use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tauri_plugin_window_state::Builder as WindowStatePlugin;
use wallet::bitcoin::{
    commands as bitcoin_commands, mnemonic as bitcoin_mnemonic, private_key as bitcoin_private_key,
    wallet as bitcoin_wallet,
};
use wallet::evm::{
    commands as evm_commands, mnemonic as evm_mnemonic, private_key as evm_private_key,
    wallet as evm_wallet,
};

pub static DB: Lazy<Mutex<db::Database>> = Lazy::new(|| {
    let db_path = if cfg!(debug_assertions) {
        "aiigo_debug.db".to_string()
    } else {
        #[cfg(target_os = "macos")]
        let data_dir = {
            if let Ok(home) = std::env::var("HOME") {
                std::path::PathBuf::from(home).join("Library/Application Support")
            } else {
                std::path::PathBuf::from(".")
            }
        };

        #[cfg(target_os = "windows")]
        let data_dir = {
            if let Ok(app_data) = std::env::var("APPDATA") {
                std::path::PathBuf::from(app_data)
            } else {
                std::path::PathBuf::from(".")
            }
        };

        #[cfg(target_os = "linux")]
        let data_dir = {
            if let Ok(home) = std::env::var("HOME") {
                std::path::PathBuf::from(home).join(".local/share")
            } else {
                std::path::PathBuf::from(".")
            }
        };

        let db_path = data_dir.join("aiigo_desktop").join("wallets.db");
        std::fs::create_dir_all(&db_path.parent().unwrap()).ok();
        db_path.to_str().unwrap().to_string()
    };

    match db::Database::new(&db_path) {
        Ok(db) => Mutex::new(db),
        Err(e) => panic!("Failed to initialize database: {}", e),
    }
});

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load environment variables from .env so runtime config (RPC URLs, WSS settings) is honored.
    let _ = dotenv();

    // Initialize database
    let _ = &*DB;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(WindowStatePlugin::default().build())
        .invoke_handler(tauri::generate_handler![
            // Bitcoin handlers
            bitcoin_mnemonic::bitcoin_create_mnemonic,
            bitcoin_mnemonic::bitcoin_import_mnemonic,
            bitcoin_wallet::bitcoin_create_wallet_from_mnemonic,
            bitcoin_private_key::bitcoin_create_wallet_from_private_key,
            bitcoin_private_key::bitcoin_export_mnemonic,
            bitcoin_private_key::bitcoin_export_private_key,
            bitcoin_commands::bitcoin_get_wallets,
            bitcoin_commands::bitcoin_get_wallet,
            bitcoin_commands::bitcoin_get_wallet_with_balance,
            bitcoin_commands::bitcoin_delete_wallet,
            // EVM handlers
            evm_mnemonic::evm_create_mnemonic,
            evm_mnemonic::evm_import_mnemonic,
            evm_wallet::evm_create_wallet_from_mnemonic,
            evm_private_key::evm_create_wallet_from_private_key,
            evm_private_key::evm_export_mnemonic,
            evm_private_key::evm_export_private_key,
            evm_commands::evm_get_wallets,
            evm_commands::evm_get_wallet,
            evm_commands::evm_get_wallet_with_balances,
            evm_commands::evm_delete_wallet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
