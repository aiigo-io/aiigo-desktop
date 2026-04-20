use crate::wallet::sync::engine;
use crate::wallet::sync::types::SyncReason;
use crate::wallet::types::{EvmWalletBalancesResponse, WalletInfo};
use crate::DB;

#[tauri::command]
pub fn evm_get_wallets() -> Result<Vec<WalletInfo>, String> {
    let db = DB.lock().unwrap();
    db.get_evm_wallets()
        .map_err(|e| format!("Failed to get wallets: {}", e))
}

#[tauri::command]
pub fn evm_get_wallet(wallet_id: String) -> Result<Option<WalletInfo>, String> {
    let db = DB.lock().unwrap();
    db.get_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to get wallet: {}", e))
}

#[tauri::command]
pub async fn evm_get_wallet_with_balances(wallet_id: String) -> Result<EvmWalletBalancesResponse, String> {
    engine::sync_evm_wallet_balances(&wallet_id, SyncReason::Manual)
        .await
        .map(|(wallet, sync)| EvmWalletBalancesResponse { wallet, sync })
}

#[cfg(test)]
mod tests {
    use super::evm_get_wallet_with_balances;
    use crate::wallet::types::EvmWalletBalancesResponse;
    use std::future::Future;

    fn assert_command_shape<F, Fut>(_command: F)
    where
        F: Fn(String) -> Fut,
        Fut: Future<Output = Result<EvmWalletBalancesResponse, String>>,
    {
    }

    #[test]
    fn evm_wallet_with_balances_command_uses_typed_partial_failure_response() {
        assert_command_shape(evm_get_wallet_with_balances);
    }
}

#[tauri::command]
pub fn evm_delete_wallet(wallet_id: String) -> Result<bool, String> {
    let db = DB.lock().unwrap();
    db.delete_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to delete wallet: {}", e))
}
