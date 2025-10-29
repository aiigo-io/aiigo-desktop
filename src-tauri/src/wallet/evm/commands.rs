use crate::wallet::types::{WalletInfo, EvmWalletInfo, EvmChainAssets, EvmAssetBalance};
use crate::wallet::evm::config::get_all_chains;
use crate::wallet::evm::balance;
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
pub async fn evm_get_wallet_with_balances(wallet_id: String) -> Result<EvmWalletInfo, String> {
    // Get wallet info first, then release the database lock
    let wallet = {
        let db = DB.lock().unwrap();
        db.get_evm_wallet(&wallet_id)
            .map_err(|e| format!("Failed to get wallet: {}", e))?
            .ok_or_else(|| "Wallet not found".to_string())?
    };
    
    // Use config to get all supported chains and their assets
    let mut chains = Vec::new();
    let total_balance_usd = 0.0;
    
    for chain_config in get_all_chains() {
        // Query balances for this chain
        let balances_result = balance::get_chain_balances(chain_config, &wallet.address).await;
        
        let mut chain_assets_vec = Vec::new();
        let chain_total_usd = 0.0;
        
        match balances_result {
            Ok(balances) => {
                let assets = chain_config.assets();
                
                for (i, asset) in assets.iter().enumerate() {
                    if i < balances.len() {
                        let (_, balance_str, balance_float) = &balances[i];
                        
                        // Save to database
                        {
                            let db = DB.lock().unwrap();
                            db.save_evm_asset_balance(
                                wallet_id.clone(),
                                chain_config.name().to_string(),
                                chain_config.chain_id(),
                                asset.symbol.clone(),
                                asset.name.clone(),
                                asset.decimals,
                                asset.contract_address.clone(),
                                balance_str.clone(),
                                *balance_float,
                                0.0,  // USD price (to be implemented)
                                0.0,  // USD value
                            ).map_err(|e| format!("Failed to save balance: {}", e))?;
                        }
                        
                        chain_assets_vec.push(EvmAssetBalance {
                            chain: chain_config.name().to_string(),
                            asset: asset.clone(),
                            balance: balance_str.clone(),
                            balance_float: *balance_float,
                        });
                    }
                }
            }
            Err(e) => {
                eprintln!("[WARNING] Failed to query balances for {}: {}", chain_config.name(), e);
                // Continue with other chains instead of failing completely
            }
        }
        
        let chain_assets = EvmChainAssets {
            chain: chain_config.name().to_string(),
            chain_id: chain_config.chain_id(),
            total_balance_usd: chain_total_usd,
            assets: chain_assets_vec,
        };
        chains.push(chain_assets);
    }
    
    Ok(EvmWalletInfo {
        id: wallet.id,
        label: wallet.label,
        wallet_type: wallet.wallet_type,
        address: wallet.address,
        chains,
        total_balance_usd,
        created_at: wallet.created_at,
        updated_at: wallet.updated_at,
    })
}

#[tauri::command]
pub fn evm_delete_wallet(wallet_id: String) -> Result<bool, String> {
    let db = DB.lock().unwrap();
    db.delete_evm_wallet(&wallet_id)
        .map_err(|e| format!("Failed to delete wallet: {}", e))
}
