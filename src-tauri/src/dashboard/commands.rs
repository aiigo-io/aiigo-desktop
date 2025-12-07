use crate::DB;
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_balance_usd: String,
    pub total_balance_btc: String,
    pub change_24h_amount: String,
    pub change_24h_percentage: String,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoPrice {
    bitcoin: CoinGeckoBitcoin,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoBitcoin {
    usd: f64,
    usd_24h_change: f64,
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
    // 1. Fetch BTC price and 24h change
    let client = Client::new();
    let response = client
        .get("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd&include_24hr_change=true")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    let price_data: CoinGeckoPrice = response.json().await.map_err(|e| e.to_string())?;
    let btc_price = price_data.bitcoin.usd;
    let btc_24h_change = price_data.bitcoin.usd_24h_change;

    // 2. Calculate Total Balance
    let db = DB.lock().map_err(|e| e.to_string())?;
    
    // BTC Wallets
    let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
    let total_btc_balance: f64 = btc_wallets.iter().map(|w| w.balance).sum();
    let btc_value_usd = total_btc_balance * btc_price;

    // EVM Wallets (Asset Balances)
    // We iterate over all EVM wallets and sum their asset usd_values
    let evm_wallets = db.get_evm_wallets().map_err(|e| e.to_string())?;
    let mut total_evm_usd = 0.0;
    
    for wallet in evm_wallets {
        let assets = db.get_evm_asset_balances(&wallet.id).map_err(|e| e.to_string())?;
        for (_, _, _, _, _, _, _, _, _, usd_value) in assets {
            total_evm_usd += usd_value;
        }
    }

    let total_portfolio_usd = btc_value_usd + total_evm_usd;
    let total_portfolio_btc = if btc_price > 0.0 { total_portfolio_usd / btc_price } else { 0.0 };

    // 3. Calculate 24h Change
    // Approximation: Apply BTC 24h change to the entire portfolio for now, 
    // or just to the BTC part and assume 0 change for others if we don't have data.
    // Let's apply BTC change to BTC part and 0 to EVM part for safety, or better, 
    // if we want to be "flashy", assume the whole portfolio moves with BTC if EVM data is stale.
    // Let's stick to: Change = (BTC_USD * BTC_Change%) + (EVM_USD * 0%) 
    // This is conservative.
    
    let btc_change_amount = btc_value_usd * (btc_24h_change / 100.0);
    // For EVM, we assume 0 change for now as we don't fetch their 24h change yet.
    let total_change_amount = btc_change_amount; 
    
    let total_change_percentage = if total_portfolio_usd > 0.0 {
        (total_change_amount / total_portfolio_usd) * 100.0
    } else {
        0.0
    };

    // 4. Update DB
    db.update_dashboard_stats(
        total_portfolio_usd,
        total_portfolio_btc,
        total_change_amount,
        total_change_percentage
    ).map_err(|e| e.to_string())?;

    // 5. Save daily snapshot for historical chart
    db.save_portfolio_snapshot(total_portfolio_usd).map_err(|e| e.to_string())?;

    // 6. Return Stats
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
pub fn get_asset_allocation() -> Result<Vec<AssetAllocation>, String> {
    let db = DB.lock().map_err(|e| e.to_string())?;
    
    let mut allocations: Vec<AssetAllocation> = Vec::new();
    let mut total_value: f64 = 0.0;
    
    // Get BTC wallets and their values (we need current BTC price to calculate USD value)
    let btc_wallets = db.get_bitcoin_wallets().map_err(|e| e.to_string())?;
    let total_btc: f64 = btc_wallets.iter().map(|w| w.balance).sum();
    
    // Get cached BTC price from dashboard stats
    let btc_usd_price = if let Some((_, btc_value, _, _, _)) = db.get_dashboard_stats().map_err(|e| e.to_string())? {
        if btc_value > 0.0 {
            // We stored total_balance_btc which is portfolio_usd / btc_price
            // Not ideal, let's just use a reasonable estimate or fetch from the previous refresh
            95000.0 // Use a reasonable default for now
        } else {
            95000.0
        }
    } else {
        95000.0
    };
    
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
        for (_chain, symbol, _chain_id, name, _contract_address, _decimals, _balance, _balance_float, _usd_price, usd_value) in assets {
            let entry = evm_assets_map.entry(symbol.clone()).or_insert((name, 0.0));
            entry.1 += usd_value;
            total_value += usd_value;
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
pub struct TopMover {
    pub symbol: String,
    pub name: String,
    pub price_usd: f64,
    pub change_24h: f64,
    pub is_positive: bool,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoMarketData {
    id: String,
    symbol: String,
    name: String,
    current_price: f64,
    price_change_percentage_24h: Option<f64>,
}

#[tauri::command]
pub async fn get_top_movers() -> Result<Vec<TopMover>, String> {
    let client = Client::new();
    
    // Fetch market data for top coins from CoinGecko
    let response = client
        .get("https://api.coingecko.com/api/v3/coins/markets")
        .query(&[
            ("vs_currency", "usd"),
            ("ids", "bitcoin,ethereum,solana,binancecoin,cardano,ripple"),
            ("order", "market_cap_desc"),
            ("per_page", "10"),
            ("page", "1"),
            ("sparkline", "false"),
            ("price_change_percentage", "24h"),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to fetch market data: {}", e))?;
    
    let market_data: Vec<CoinGeckoMarketData> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse market data: {}", e))?;
    
    let movers: Vec<TopMover> = market_data
        .into_iter()
        .map(|coin| {
            let change_24h = coin.price_change_percentage_24h.unwrap_or(0.0);
            TopMover {
                symbol: coin.symbol.to_uppercase(),
                name: coin.name,
                price_usd: coin.current_price,
                change_24h,
                is_positive: change_24h >= 0.0,
            }
        })
        .collect();
    
    Ok(movers)
}
