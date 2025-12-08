use crate::db::AssetBalanceData;
use crate::wallet::evm::balance;
use crate::wallet::evm::config::{chain_concurrency_limit, get_all_chains};
use crate::wallet::evm::price;
use crate::wallet::types::{EvmAssetBalance, EvmChainAssets, EvmWalletInfo, WalletInfo};
use crate::DB;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

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

    let chains_config = get_all_chains();

    // Collect all unique asset symbols across all chains for batch price fetching
    let symbol_set: HashSet<String> = chains_config
        .iter()
        .flat_map(|chain| chain.assets())
        .map(|asset| asset.symbol.clone())
        .collect();
    let all_symbols: Vec<String> = symbol_set.into_iter().collect();

    // Fetch all prices at once
    let prices_map = price::fetch_prices(all_symbols).await.unwrap_or_default();
    let prices = Arc::new(prices_map);

    let concurrency_limit = chain_concurrency_limit();
    let semaphore = Arc::new(Semaphore::new(concurrency_limit));
    let wallet_address = wallet.address.clone();
    let mut set = JoinSet::new();

    for (order, chain_config) in chains_config.into_iter().enumerate() {
        let semaphore = semaphore.clone();
        let prices = prices.clone();
        let address = wallet_address.clone();
        let wallet_id_cloned = wallet_id.clone();

        set.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .map_err(|e| format!("Semaphore closed: {}", e))?;

            let chain_name = chain_config.name().to_string();
            let chain_id = chain_config.chain_id();

            let balances_result = balance::get_chain_balances(chain_config, &address).await;
            let mut chain_assets_vec = Vec::new();
            let mut chain_total_usd = 0.0;
            let mut asset_data_for_db = Vec::new();

            match balances_result {
                Ok(balances) => {
                    let balance_map: HashMap<String, (String, f64)> = balances
                        .into_iter()
                        .map(|(symbol, balance_str, balance_float)| {
                            (symbol, (balance_str, balance_float))
                        })
                        .collect();

                    for asset in chain_config.assets() {
                        if let Some((balance_str, balance_float)) = balance_map.get(&asset.symbol) {
                            let usd_price = prices.get(&asset.symbol).copied().unwrap_or(0.0);
                            let usd_value = balance_float * usd_price;
                            chain_total_usd += usd_value;

                            asset_data_for_db.push(AssetBalanceData {
                                wallet_id: wallet_id_cloned.clone(),
                                chain: chain_name.clone(),
                                chain_id,
                                symbol: asset.symbol.clone(),
                                name: asset.name.clone(),
                                decimals: asset.decimals,
                                contract_address: asset.contract_address.clone(),
                                balance: balance_str.clone(),
                                balance_float: *balance_float,
                                usd_price,
                                usd_value,
                            });

                            chain_assets_vec.push(EvmAssetBalance {
                                chain: chain_name.clone(),
                                asset: asset.clone(),
                                balance: balance_str.clone(),
                                balance_float: *balance_float,
                                usd_price,
                                usd_value,
                            });
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[WARNING] Failed to query balances for {}: {}",
                        chain_name, e
                    );
                }
            }

            Ok::<_, String>((
                order,
                EvmChainAssets {
                    chain: chain_name,
                    chain_id,
                    total_balance_usd: chain_total_usd,
                    assets: chain_assets_vec,
                },
                asset_data_for_db,
            ))
        });
    }

    let mut chains_with_order = Vec::new();
    let mut all_asset_data = Vec::new();
    let mut total_balance_usd = 0.0;

    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok((order, chain_assets, asset_data))) => {
                total_balance_usd += chain_assets.total_balance_usd;
                chains_with_order.push((order, chain_assets));
                all_asset_data.extend(asset_data);
            }
            Ok(Err(e)) => eprintln!("[ERROR] Chain balance task failed: {}", e),
            Err(join_err) => eprintln!("[ERROR] Chain task panicked: {:?}", join_err),
        }
    }

    chains_with_order.sort_by_key(|(order, _)| *order);
    let chains: Vec<EvmChainAssets> = chains_with_order
        .into_iter()
        .map(|(_, chain)| chain)
        .collect();

    if !all_asset_data.is_empty() {
        let db = DB.lock().unwrap();
        db.batch_save_evm_asset_balances(&all_asset_data)
            .map_err(|e| format!("Failed to save balances: {}", e))?;
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
