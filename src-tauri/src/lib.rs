mod db;
mod wallet;
mod dashboard;

use wallet::bitcoin::{mnemonic as bitcoin_mnemonic, wallet as bitcoin_wallet, commands as bitcoin_commands, private_key as bitcoin_private_key};
use wallet::evm::{mnemonic as evm_mnemonic, wallet as evm_wallet, commands as evm_commands, private_key as evm_private_key};
use wallet::security::backend::SecretBackend;
use wallet::security::commands::{security_get_backend_state, security_has_password, security_is_unlocked, security_lock, security_setup_password, security_unlock, AppSecurity, StartupSecurityState};
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

fn build_app_security(secret_backend: Arc<SecretBackend>) -> AppSecurity {
    AppSecurity {
        session_manager: Arc::new(SessionManager::new(Duration::from_secs(300))),
        keystore: Arc::new(SqliteKeystore::new(&DB, secret_backend.clone()))
            as Arc<dyn Keystore + Send + Sync>,
        secret_backend,
        startup_state: Arc::new(Mutex::new(StartupSecurityState::default())),
    }
}

fn initialize_security_startup_state(app_security: &AppSecurity) {
    let mut has_legacy_plaintext_secrets = false;

    if let Ok(db) = DB.lock() {
        let legacy_rows = db.count_legacy_secret_rows_total().unwrap_or(0);
        has_legacy_plaintext_secrets = legacy_rows > 0;
    }

    if let Ok(mut startup_state) = app_security.startup_state().lock() {
        startup_state.has_legacy_plaintext_secrets = has_legacy_plaintext_secrets;
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = dotenv();
    init_tracing();

    let _ = &*DB;
    let secret_backend = Arc::new(SecretBackend::new());
    let app_security = build_app_security(secret_backend);
    initialize_security_startup_state(&app_security);

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
            bitcoin_commands::query_bitcoin_wallet_balance,
            bitcoin_commands::refresh_bitcoin_wallet_balance,
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
            evm_commands::query_evm_wallet_balances,
            evm_commands::refresh_evm_wallet_balances,
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
            security_has_password,
            security_setup_password,
            security_unlock,
            security_lock,
            security_is_unlocked,
            security_get_backend_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{build_app_security, initialize_security_startup_state};
    use crate::wallet::security::backend::{SecretBackend, SecretBackendAdapter};
    use crate::wallet::security::secret_envelope::{SecretEnvelopeError, StoredSecret};
    use crate::wallet::security::types::SecretBackendStatus;
    use std::sync::Arc;

    struct PanicOnProbeAdapter;

    impl SecretBackendAdapter for PanicOnProbeAdapter {
        fn probe(&self) -> Result<(), SecretEnvelopeError> {
            panic!("startup should not probe the secret backend");
        }

        fn encrypt(&self, _plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
            unreachable!("test should not encrypt during startup wiring");
        }

        fn decrypt(&self, _secret_data: &str, _secret_format: &str) -> Result<String, SecretEnvelopeError> {
            unreachable!("test should not decrypt during startup wiring");
        }
    }

    #[test]
    fn building_app_security_does_not_probe_secret_backend() {
        let secret_backend = Arc::new(SecretBackend::with_adapter(Arc::new(PanicOnProbeAdapter)));

        let app_security = build_app_security(secret_backend.clone());

        assert!(matches!(
            app_security.secret_backend().current_status(),
            SecretBackendStatus::Unknown
        ));
    }

    #[test]
    fn startup_state_initialization_does_not_probe_secret_backend() {
        let secret_backend = Arc::new(SecretBackend::with_adapter(Arc::new(PanicOnProbeAdapter)));
        let app_security = build_app_security(secret_backend);

        initialize_security_startup_state(&app_security);

        assert!(matches!(
            app_security.secret_backend().current_status(),
            SecretBackendStatus::Unknown
        ));
    }
}
