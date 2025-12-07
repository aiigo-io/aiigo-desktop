mod wallet;
mod db;
mod dashboard;

use wallet::bitcoin::{mnemonic as bitcoin_mnemonic, wallet as bitcoin_wallet, commands as bitcoin_commands, private_key as bitcoin_private_key};
use wallet::evm::{mnemonic as evm_mnemonic, wallet as evm_wallet, commands as evm_commands, private_key as evm_private_key};
use wallet::transaction_commands;
use tauri_plugin_window_state::Builder as WindowStatePlugin;
use std::sync::Mutex;
use once_cell::sync::Lazy;

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
            evm_commands::evm_get_wallet_with_balances,
            evm_commands::evm_delete_wallet,
            // Transaction handlers
            transaction_commands::send_bitcoin,
            transaction_commands::get_bitcoin_transactions,
            transaction_commands::get_all_bitcoin_transactions,
            transaction_commands::fetch_bitcoin_history,
            transaction_commands::send_evm,
            transaction_commands::get_evm_transactions,
            transaction_commands::get_all_evm_transactions,
            transaction_commands::fetch_evm_history,
            // Dashboard handlers
            dashboard::commands::get_dashboard_stats,
            dashboard::commands::refresh_dashboard_stats,
            dashboard::commands::get_portfolio_history,
            dashboard::commands::get_asset_allocation,
            dashboard::commands::get_top_movers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
