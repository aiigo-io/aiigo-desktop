mod compute;
mod dashboard;
mod db;
mod wallet;

use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::{env, fs, path::Path, path::PathBuf, time::SystemTime};
use tauri_plugin_window_state::Builder as WindowStatePlugin;
use wallet::bitcoin::{
    commands as bitcoin_commands, mnemonic as bitcoin_mnemonic, private_key as bitcoin_private_key,
    wallet as bitcoin_wallet,
};
use wallet::evm::{
    commands as evm_commands, mnemonic as evm_mnemonic, private_key as evm_private_key,
    wallet as evm_wallet,
};
use wallet::security::backend::SecretBackend;
use wallet::security::commands::{
    security_authorize_operation, security_get_backend_state, security_get_local_password_policy,
    security_has_password, security_is_unlocked, security_lock, security_probe_backend,
    security_reset_local_wallet_data, security_setup_password, security_unlock, AppSecurity,
    StartupSecurityState,
};
use wallet::security::keystore::{Keystore, SqliteKeystore};
use wallet::security::session::SessionManager;
use wallet::security::types::{
    LOCAL_PASSWORD_IDLE_LOCK_SECONDS, LOCAL_PASSWORD_REAUTH_WINDOW_SECONDS,
};
use wallet::state::commands as state_commands;
use wallet::transaction_commands;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Debug, Clone)]
struct LegacyDbCandidate {
    path: PathBuf,
    size: u64,
    modified_at: SystemTime,
}

pub static DB: Lazy<Mutex<db::Database>> = Lazy::new(|| {
    #[cfg(test)]
    let database = db::Database::new(":memory:")
        .unwrap_or_else(|e| panic!("Failed to initialize in-memory test database: {}", e));

    #[cfg(not(test))]
    let database = {
        let db_path = resolve_db_path();
        db::Database::new(&db_path.to_string_lossy())
            .unwrap_or_else(|e| panic!("Failed to initialize database: {}", e))
    };

    Mutex::new(database)
});

fn resolve_db_path() -> PathBuf {
    let db_path = stable_db_path(cfg!(debug_assertions));

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).ok();
    }

    if cfg!(debug_assertions) {
        migrate_legacy_debug_db_if_needed(&db_path);
    }

    db_path
}

fn stable_db_path(debug: bool) -> PathBuf {
    let file_name = if debug { "aiigo_debug.db" } else { "wallets.db" };
    app_data_dir().join("aiigo_desktop").join(file_name)
}

fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join("Library/Application Support");
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = env::var("APPDATA") {
            return PathBuf::from(app_data);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join(".local/share");
        }
    }

    PathBuf::from(".")
}

fn migrate_legacy_debug_db_if_needed(target_path: &Path) {
    if target_has_user_data(target_path) {
        return;
    }

    let Some(source) = select_legacy_debug_db_candidate() else {
        return;
    };

    if source.path == target_path {
        return;
    }

    match fs::copy(&source.path, target_path) {
        Ok(_) => {
            tracing::info!(
                source = %source.path.display(),
                target = %target_path.display(),
                "migrated legacy debug database to stable application support path"
            );
        }
        Err(error) => {
            tracing::warn!(
                source = %source.path.display(),
                target = %target_path.display(),
                %error,
                "failed to migrate legacy debug database"
            );
        }
    }
}

fn target_has_user_data(path: &Path) -> bool {
    fs::metadata(path).map(|meta| meta.len() > 0).unwrap_or(false)
}

fn select_legacy_debug_db_candidate() -> Option<LegacyDbCandidate> {
    legacy_debug_db_candidates()
        .into_iter()
        .filter_map(|path| {
            let metadata = fs::metadata(&path).ok()?;
            if !metadata.is_file() || metadata.len() == 0 {
                return None;
            }

            Some(LegacyDbCandidate {
                path,
                size: metadata.len(),
                modified_at: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            })
        })
        .max_by(|left, right| {
            left.modified_at
                .cmp(&right.modified_at)
                .then(left.size.cmp(&right.size))
        })
}

fn legacy_debug_db_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("aiigo_debug.db"));
        candidates.push(cwd.join("src-tauri").join("aiigo_debug.db"));

        if let Some(parent) = cwd.parent() {
            candidates.push(parent.join("aiigo_debug.db"));
            candidates.push(parent.join("src-tauri").join("aiigo_debug.db"));
        }
    }

    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.iter().any(|existing| existing == &candidate) {
            unique.push(candidate);
        }
    }
    unique
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(false)
                .with_file(true)
                .with_line_number(true),
        )
        .init();
}

fn build_app_security(secret_backend: Arc<SecretBackend>) -> AppSecurity {
    AppSecurity {
        session_manager: Arc::new(SessionManager::new(
            Duration::from_secs(LOCAL_PASSWORD_IDLE_LOCK_SECONDS),
            Duration::from_secs(LOCAL_PASSWORD_REAUTH_WINDOW_SECONDS),
        )),
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
            transaction_commands::get_supported_evm_history_chains,
            transaction_commands::evm_send_transaction,
            transaction_commands::evm_approve_token,
            transaction_commands::refresh_evm_transaction_lifecycle,
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
            security_probe_backend,
            security_get_local_password_policy,
            security_authorize_operation,
            security_reset_local_wallet_data,
            // Compute marketplace handlers
            compute::commands::compute_get_config,
            compute::commands::query_compute_marketplace_snapshot,
            compute::commands::refresh_compute_marketplace_snapshot,
            compute::commands::compute_register_node,
            compute::commands::compute_verify_node,
            compute::commands::compute_create_and_fund_task,
            compute::commands::compute_accept_task,
            compute::commands::compute_submit_result,
            compute::commands::compute_approve_task,
            compute::commands::compute_dispute_task,
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

        fn initialize_empty_store(&self) -> Result<(), SecretEnvelopeError> {
            unreachable!("test should not initialize empty-store keyring state during startup wiring");
        }

        fn encrypt(&self, _plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
            unreachable!("test should not encrypt during startup wiring");
        }

        fn decrypt(
            &self,
            _secret_data: &str,
            _secret_format: &str,
        ) -> Result<String, SecretEnvelopeError> {
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
