use crate::DB;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_balance_usd: String,
    pub total_balance_btc: String,
    pub change_24h_amount: String,
    pub change_24h_percentage: String,
}

// Helper function to format numbers with thousand separators
fn format_currency(value: f64) -> String {
    let abs_value = value.abs();
    let formatted = format!("{:.2}", abs_value);
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = if parts.len() > 1 { parts[1] } else { "00" };
    
    // Add thousand separators
    let mut result = String::new();
    for (i, c) in integer_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    
    let formatted_integer: String = result.chars().rev().collect();
    format!("{}.{}", formatted_integer, decimal_part)
}

#[tauri::command]
pub fn get_dashboard_stats() -> Result<DashboardStats, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let stats = db.get_dashboard_stats().map_err(|e| e.to_string())?;

    if let Some((usd, btc, change_amt, change_pct, _)) = stats {
        Ok(DashboardStats {
            total_balance_usd: format!("${}", format_currency(usd)),
            total_balance_btc: format!("≈ {:.4} BTC", btc),
            change_24h_amount: format!("{}${}", if change_amt >= 0.0 { "+" } else { "" }, format_currency(change_amt)),
            change_24h_percentage: format!("{}{:.2}%", if change_pct >= 0.0 { "+" } else { "" }, change_pct),
        })
    } else {
        Ok(DashboardStats {
            total_balance_usd: "$0.00".to_string(),
            total_balance_btc: "≈ 0.0000 BTC".to_string(),
            change_24h_amount: "+$0.00".to_string(),
            change_24h_percentage: "+0.00%".to_string(),
        })
    }
}

#[tauri::command]
pub async fn refresh_dashboard_stats() -> Result<DashboardStats, String> {
    // 1. Get BTC price from cache (background task keeps it fresh)
    let btc_price = crate::wallet::evm::price_manager::get_cached_price("BTC")
        .unwrap_or(95000.0); // Fallback if cache not ready yet
    
    // Use a scope to drop the lock before fetching fresh EVM prices
    let (total_btc_balance, wallet_assets_map, symbols_to_price) = {
        let db = DB.lock().map_err(|e| e.to_string())?;
        
        // 2. Refresh BTC Balance
        let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
        let total_btc_balance: f64 = btc_wallets.iter().map(|w| w.balance).sum();

        // 3. Prepare EVM Balances and Prices
        let evm_wallets = db.get_evm_wallets().map_err(|e| e.to_string())?;
        let mut wallet_assets_map = Vec::new(); // (wallet_id, assets)
        let mut symbols_to_price = std::collections::HashSet::new();

        for wallet in &evm_wallets {
            let assets = db.get_evm_asset_balances(&wallet.id).map_err(|e| e.to_string())?;
            for asset in &assets {
                symbols_to_price.insert(asset.1.clone()); // symbol
            }
            wallet_assets_map.push((wallet.id.clone(), assets));
        }
        (total_btc_balance, wallet_assets_map, symbols_to_price)
    };

    let btc_value_usd = total_btc_balance * btc_price;

    // Fetch fresh prices for all EVM symbols from cache (Lock is dropped here)
    let mut fresh_prices = std::collections::HashMap::new();
    for symbol in symbols_to_price {
        if let Some(price) = crate::wallet::evm::price_manager::get_cached_price(&symbol) {
            fresh_prices.insert(symbol, price);
        }
    }

    let mut total_evm_usd = 0.0;
    let mut all_assets_to_update = Vec::new();
    
    // Update EVM assets with fresh prices and sum them up
    for (wallet_id, assets) in wallet_assets_map {
        for asset in assets {
            let chain_id = asset.2;
            let symbol = &asset.1;
            
            let mut usd_price = asset.8;
            let mut usd_value = asset.9;

            if let Some(&fresh_price) = fresh_prices.get(symbol) {
                usd_price = fresh_price;
                usd_value = asset.7 * fresh_price; // balance_float * fresh_price
            }

            if chain_id != 11155111 {
                total_evm_usd += usd_value;
            }

            // Prepare for DB update
            all_assets_to_update.push(crate::db::AssetBalanceData {
                wallet_id: wallet_id.clone(),
                chain: asset.0,
                chain_id,
                symbol: symbol.clone(),
                name: asset.3,
                decimals: asset.5,
                contract_address: asset.4,
                balance: asset.6,
                balance_float: asset.7,
                usd_price,
                usd_value,
            });
        }
    }

    let total_portfolio_usd = btc_value_usd + total_evm_usd;
    let total_portfolio_btc = if btc_price > 0.0 { total_portfolio_usd / btc_price } else { 0.0 };

    // 4. Calculate 24h Change (Include both BTC and EVM)
    // BTC contribution
    let btc_24h_change = crate::wallet::evm::price_manager::get_cached_24h_change("BTC")
        .unwrap_or(0.0);
    let btc_change_amount = btc_value_usd * (btc_24h_change / 100.0);
    
    // EVM contribution
    let mut evm_change_amount = 0.0;
    for asset in &all_assets_to_update {
        // Exclude testnet/untracked assets (though price_manager should handle them)
        if let Some(change) = crate::wallet::evm::price_manager::get_cached_24h_change(&asset.symbol) {
            evm_change_amount += asset.usd_value * (change / 100.0);
        }
    }

    let total_change_amount = btc_change_amount + evm_change_amount; 
    
    let total_change_percentage = if total_portfolio_usd > 0.0 {
        (total_change_amount / total_portfolio_usd) * 100.0
    } else {
        0.0
    };

    // 5. Update DB (Re-acquire lock)
    {
        let db = DB.lock().map_err(|e| e.to_string())?;
        
        // Update DB with fresh prices
        if !all_assets_to_update.is_empty() {
            db.batch_save_evm_asset_balances(&all_assets_to_update)
                .map_err(|e| format!("Failed to update fresh prices in DB: {}", e))?;
        }

        db.update_dashboard_stats(
            total_portfolio_usd,
            total_portfolio_btc,
            total_change_amount,
            total_change_percentage
        ).map_err(|e| e.to_string())?;

        // 6. Save daily snapshot for historical chart
        db.save_portfolio_snapshot(total_portfolio_usd).map_err(|e| e.to_string())?;
    }

    // 7. Return Stats
    Ok(DashboardStats {
        total_balance_usd: format!("${}", format_currency(total_portfolio_usd)),
        total_balance_btc: format!("≈ {:.4} BTC", total_portfolio_btc),
        change_24h_amount: format!("{}${}", if total_change_amount >= 0.0 { "+" } else { "" }, format_currency(total_change_amount)),
        change_24h_percentage: format!("{}{:.2}%", if total_change_percentage >= 0.0 { "+" } else { "" }, total_change_percentage),
    })
}

#[derive(Debug, Serialize)]
pub struct PortfolioHistoryPoint {
    pub date: String,
    pub value: f64,
}

#[tauri::command]
pub fn get_portfolio_history() -> Result<Vec<PortfolioHistoryPoint>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    let history = db.get_portfolio_history(7).map_err(|e| e.to_string())?;
    
    let result = history.into_iter().map(|(date, value)| {
        PortfolioHistoryPoint { date, value }
    }).collect();
    
    Ok(result)
}

#[derive(Debug, Serialize)]
pub struct AssetAllocation {
    pub name: String,
    pub symbol: String,
    pub percentage: f64,
    pub value_usd: f64,
    pub color: String,
}

#[tauri::command]
pub async fn get_asset_allocation() -> Result<Vec<AssetAllocation>, String> {
    // Get BTC price from cache (background task keeps it fresh)
    let btc_usd_price = crate::wallet::evm::price_manager::get_cached_price("BTC")
        .unwrap_or(95000.0); // Fallback if cache not ready yet

    let db = DB.lock().map_err(|e| e.to_string())?;
    
    let mut allocations: Vec<AssetAllocation> = Vec::new();
    let mut total_value: f64 = 0.0;
    
    // Get BTC wallets and their values
    let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
    let total_btc: f64 = btc_wallets.iter().map(|w| w.balance).sum();
    
    let btc_value_usd = total_btc * btc_usd_price;
    if btc_value_usd > 0.0 {
        allocations.push(AssetAllocation {
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            percentage: 0.0, // Will be calculated after getting total
            value_usd: btc_value_usd,
            color: "bg-orange-500".to_string(),
        });
        total_value += btc_value_usd;
    }
    
    // Get EVM assets
    let evm_wallets = db.get_evm_wallets().map_err(|e| e.to_string())?;
    let mut evm_assets_map: std::collections::HashMap<String, (String, f64)> = std::collections::HashMap::new();
    
    for wallet in evm_wallets {
        let assets = db.get_evm_asset_balances(&wallet.id).map_err(|e| e.to_string())?;
        // Tuple: (chain, asset_symbol, chain_id, asset_name, contract_address, asset_decimals, balance, balance_float, usd_price, usd_value)
        for (_chain, symbol, chain_id, name, _contract_address, _decimals, _balance, _balance_float, _usd_price, usd_value) in assets {
            // Exclude Sepolia
            if chain_id != 11155111 {
                let entry = evm_assets_map.entry(symbol.clone()).or_insert((name, 0.0));
                entry.1 += usd_value;
                total_value += usd_value;
            }
        }
    }
    
    // Add EVM assets to allocations
    let colors = ["bg-blue-500", "bg-purple-500", "bg-cyan-500", "bg-green-500", "bg-pink-500", "bg-indigo-500"];
    let mut color_index = 0;
    
    for (symbol, (name, usd_value)) in evm_assets_map {
        if usd_value > 0.0 {
            allocations.push(AssetAllocation {
                name,
                symbol,
                percentage: 0.0,
                value_usd: usd_value,
                color: colors[color_index % colors.len()].to_string(),
            });
            color_index += 1;
        }
    }
    
    // Calculate percentages
    if total_value > 0.0 {
        for allocation in &mut allocations {
            allocation.percentage = (allocation.value_usd / total_value) * 100.0;
        }
    }
    
    // Sort by value (descending)
    allocations.sort_by(|a, b| b.value_usd.partial_cmp(&a.value_usd).unwrap_or(std::cmp::Ordering::Equal));
    
    // Limit to top 5 and group the rest as "Other"
    if allocations.len() > 5 {
        let top_5: Vec<AssetAllocation> = allocations.drain(..5).collect();
        let other_value: f64 = allocations.iter().map(|a| a.value_usd).sum();
        let other_percentage = if total_value > 0.0 { (other_value / total_value) * 100.0 } else { 0.0 };
        
        allocations = top_5;
        if other_value > 0.0 {
            allocations.push(AssetAllocation {
                name: "Other".to_string(),
                symbol: "OTHER".to_string(),
                percentage: other_percentage,
                value_usd: other_value,
                color: "bg-slate-400".to_string(),
            });
        }
    }
    
    Ok(allocations)
}


#[derive(Debug, Serialize)]
pub struct UnifiedTransaction {
    pub id: String,
    pub r#type: String, // "bitcoin" or "evm"
    pub tx_type: String, // "send", "receive", etc.
    pub tx_hash: String,
    pub asset_symbol: String,
    pub amount: String,
    pub timestamp: String,
}

#[tauri::command]
pub fn get_unified_recent_transactions() -> Result<Vec<UnifiedTransaction>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;

    // 1. Get Bitcoin Transactions
    let btc_txs = db.get_all_bitcoin_transactions().map_err(|e| e.to_string())?;
    let mut unified: Vec<UnifiedTransaction> = btc_txs
        .into_iter()
        .filter(|tx| tx.status.as_str() != "failed")
        .map(|tx| UnifiedTransaction {
            id: tx.id,
            r#type: "bitcoin".to_string(),
            tx_type: tx.tx_type.as_str().to_string(),
            tx_hash: tx.tx_hash,
            asset_symbol: "BTC".to_string(),
            amount: format!("{:.8}", tx.amount),
            timestamp: tx.timestamp,
        })
        .collect();

    // 2. Get EVM Transactions
    let evm_txs = db.get_all_evm_transactions().map_err(|e| e.to_string())?;
    let evm_unified: Vec<UnifiedTransaction> = evm_txs
        .into_iter()
        .filter(|tx| tx.status.as_str() != "failed" && tx.chain_id != 11155111) // Filter out Sepolia
        .map(|tx| UnifiedTransaction {
            id: tx.id,
            r#type: "evm".to_string(),
            tx_type: tx.tx_type.as_str().to_string(),
            tx_hash: tx.tx_hash,
            asset_symbol: tx.asset_symbol,
            amount: format!("{:.6}", tx.amount_float),
            timestamp: tx.timestamp,
        })
        .collect();

    unified.extend(evm_unified);

    // 3. Sort by timestamp descending
    unified.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // 4. Return top 5
    if unified.len() > 5 {
        unified.truncate(5);
    }

    Ok(unified)
}

#[derive(Debug, Serialize)]
pub struct UnifiedAsset {
    pub name: String,
    pub symbol: String,
    pub balance: f64,
    pub usd_value: f64,
    pub chain: String, // "Bitcoin" or EVM chain name
    pub is_testnet: bool,
}

#[tauri::command]
pub fn get_unified_assets() -> Result<Vec<UnifiedAsset>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();

    // 1. BTC Wallets
    let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
    let total_btc_balance: f64 = btc_wallets.iter().map(|w| w.balance).sum();
    
    if total_btc_balance > 0.0 {
        // Get BTC price from cache (background task keeps it fresh)
        let btc_price = crate::wallet::evm::price_manager::get_cached_price("BTC")
            .unwrap_or(95000.0); // Fallback if cache not ready yet

        result.push(UnifiedAsset {
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            balance: total_btc_balance,
            usd_value: total_btc_balance * btc_price,
            chain: "Bitcoin".to_string(),
            is_testnet: false,
        });
    }

    // 2. EVM Assets
    let evm_wallets = db.get_evm_wallets().map_err(|e| e.to_string())?;
    for wallet in evm_wallets {
        let assets = db.get_evm_asset_balances(&wallet.id).map_err(|e| e.to_string())?;
        for (chain, symbol, chain_id, name, _, _, _, balance_float, _, usd_value) in assets {
            result.push(UnifiedAsset {
                name,
                symbol,
                balance: balance_float,
                usd_value,
                chain,
                is_testnet: chain_id == 11155111,
            });
        }
    }

    Ok(result)
}
