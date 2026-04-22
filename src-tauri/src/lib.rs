mod db;
mod wallet;
mod dashboard;

use wallet::bitcoin::{mnemonic as bitcoin_mnemonic, wallet as bitcoin_wallet, commands as bitcoin_commands, private_key as bitcoin_private_key};
use wallet::evm::{mnemonic as evm_mnemonic, wallet as evm_wallet, commands as evm_commands, private_key as evm_private_key};
use wallet::security::backend::SecretBackend;
use wallet::security::commands::{security_is_unlocked, security_lock, security_unlock, AppSecurity};
use wallet::security::keystore::{Keystore, SqliteKeystore};
use wallet::security::session::SessionManager;
use wallet::state::commands as state_commands;
use wallet::transaction_commands;
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tauri_plugin_window_state::Builder as WindowStatePlugin;

use tracing_subscriber::{fmt, EnvFilter, prelude::*};

enum SecretMigrationLogEntry {
    Info(String),
    Error(String),
}

fn build_secret_migration_log_entry(
    result: Result<db::SecretMigrationReport, String>,
) -> SecretMigrationLogEntry {
    match result {
        Ok(report) => SecretMigrationLogEntry::Info(format!(
            "Secret migration attempted={} migrated={} skipped={} failed={}",
            report.attempted_rows, report.migrated_rows, report.skipped_rows, report.failed_rows
        )),
        Err(error) => SecretMigrationLogEntry::Error(error),
    }
}

fn emit_secret_migration_log(entry: SecretMigrationLogEntry) {
    match entry {
        SecretMigrationLogEntry::Info(message) => safe_log!("[INFO] {}", message),
        SecretMigrationLogEntry::Error(message) => safe_log!("[ERROR] {}", message),
    }
}

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

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).with_file(true).with_line_number(true))
        .init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = dotenv();
    init_tracing();

    let _ = &*DB;
    let secret_backend = Arc::new(SecretBackend::new());
    let app_security = AppSecurity {
        session_manager: Arc::new(SessionManager::new(Duration::from_secs(300))),
        keystore: Arc::new(SqliteKeystore::new(&DB, secret_backend.clone())) as Arc<dyn Keystore + Send + Sync>,
        secret_backend: secret_backend.clone(),
    };

    let _ = secret_backend.refresh_status();
    let migration_log_entry = match DB.lock() {
        Ok(db) => build_secret_migration_log_entry(
            db.run_secret_storage_migration(secret_backend.as_ref())
                .map_err(|error| format!("Secret migration failed: {}", error)),
        ),
        Err(error) => build_secret_migration_log_entry(Err(format!(
            "Failed to acquire database lock for secret migration: {}",
            error
        ))),
    };
    emit_secret_migration_log(migration_log_entry);

    tauri::Builder::default()
        .manage(app_security)
        .plugin(tauri_plugin_opener::init())
        .plugin(WindowStatePlugin::default().build())
        .setup(|_app| {
            // Initialize price manager with background refresh
            tauri::async_runtime::spawn(async move {
                wallet::evm::price_manager::start_background_refresh().await;
            });
            
            Ok(())
        })
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
            // Transaction handlers
            transaction_commands::send_bitcoin,
            transaction_commands::get_bitcoin_transactions,
            transaction_commands::get_all_bitcoin_transactions,
            transaction_commands::fetch_bitcoin_history,
            transaction_commands::bitcoin_estimate_fees,
            transaction_commands::send_evm,
            transaction_commands::evm_estimate_gas,
            transaction_commands::get_evm_transactions,
            transaction_commands::get_all_evm_transactions,
            transaction_commands::fetch_evm_history,
            transaction_commands::evm_send_transaction,
            transaction_commands::evm_approve_token,
            // Dashboard handlers
            dashboard::commands::get_dashboard_stats,
            dashboard::commands::refresh_dashboard_stats,
            dashboard::commands::get_portfolio_history,
            dashboard::commands::get_asset_allocation,
            dashboard::commands::get_unified_recent_transactions,
            wallet::evm::price::get_bitcoin_price,
            // State handlers
            state_commands::state_get_bitcoin_wallet_balance_state,
            state_commands::state_get_bitcoin_price_state,
            state_commands::state_get_bitcoin_portfolio_state,
            // Security handlers
            security_unlock,
            security_lock,
            security_is_unlocked,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{build_secret_migration_log_entry, SecretMigrationLogEntry};
    use crate::db::SecretMigrationReport;

    #[test]
    fn secret_migration_success_produces_info_entry() {
        let entry = build_secret_migration_log_entry(Ok(SecretMigrationReport {
            attempted_rows: 3,
            migrated_rows: 2,
            skipped_rows: 1,
            failed_rows: 0,
        }));

        match entry {
            SecretMigrationLogEntry::Info(message) => {
                assert_eq!(
                    message,
                    "Secret migration attempted=3 migrated=2 skipped=1 failed=0"
                );
            }
            SecretMigrationLogEntry::Error(message) => {
                panic!("expected info entry, got error: {}", message);
            }
        }
    }

    #[test]
    fn secret_migration_failure_produces_error_entry() {
        let entry = build_secret_migration_log_entry(Err(
            "Secret migration failed: database is locked".to_string(),
        ));

        match entry {
            SecretMigrationLogEntry::Info(message) => {
                panic!("expected error entry, got info: {}", message);
            }
            SecretMigrationLogEntry::Error(message) => {
                assert_eq!(message, "Secret migration failed: database is locked");
            }
        }
    }
}
