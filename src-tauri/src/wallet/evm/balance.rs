use crate::wallet::evm::config::EvmChainConfig;
use ethers::prelude::*;
use std::time::Duration;

const RETRY_ATTEMPTS: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 500;
const REQUEST_DELAY_MS: u64 = 100;  // Delay between requests to avoid rate limiting

/// Query ETH balance for an address with retry logic
pub async fn query_eth_balance(
    chain_config: EvmChainConfig,
    address: &str,
) -> Result<String, String> {
    let address: Address = address.parse()
        .map_err(|_| "Invalid address format".to_string())?;
    
    let rpc_url = chain_config.rpc_url();
    
    for attempt in 1..=RETRY_ATTEMPTS {
        match try_query_eth_balance(&rpc_url, address).await {
            Ok(balance) => return Ok(balance),
            Err(e) => {
                if attempt < RETRY_ATTEMPTS {
                    let delay_ms = INITIAL_RETRY_DELAY_MS * (2_u64.pow(attempt - 1));
                    eprintln!("[RETRY] ETH balance query attempt {} failed: {}. Retrying in {}ms...", attempt, e, delay_ms);
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
    
    Err("Failed to query balance after all retries".to_string())
}

async fn try_query_eth_balance(rpc_url: &str, address: Address) -> Result<String, String> {
    let provider = Provider::<Http>::try_from(rpc_url)
        .map_err(|e| format!("Failed to connect to RPC: {}", e))?;
    
    let balance = provider
        .get_balance(address, None)
        .await
        .map_err(|e| format!("Failed to query balance: {}", e))?;
    
    Ok(balance.to_string())
}

/// Query ERC20 token balance for an address with retry logic
pub async fn query_erc20_balance(
    chain_config: EvmChainConfig,
    token_address: &str,
    wallet_address: &str,
) -> Result<String, String> {
    let token_addr: Address = token_address.parse()
        .map_err(|_| "Invalid token address".to_string())?;
    
    let wallet_addr: Address = wallet_address.parse()
        .map_err(|_| "Invalid wallet address".to_string())?;
    
    let rpc_url = chain_config.rpc_url();
    
    for attempt in 1..=RETRY_ATTEMPTS {
        match try_query_erc20_balance(&rpc_url, chain_config.name(), token_addr, wallet_addr).await {
            Ok(balance) => return Ok(balance),
            Err(e) => {
                if attempt < RETRY_ATTEMPTS {
                    let delay_ms = INITIAL_RETRY_DELAY_MS * (2_u64.pow(attempt - 1));
                    eprintln!("[RETRY] ERC20 balance query attempt {} failed: {}. Retrying in {}ms...", attempt, e, delay_ms);
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
    
    Err("Failed to query balance after all retries".to_string())
}

async fn try_query_erc20_balance(rpc_url: &str, chain_name: &str, token_addr: Address, wallet_addr: Address) -> Result<String, String> {
    let provider = Provider::<Http>::try_from(rpc_url)
        .map_err(|e| format!("Failed to connect to RPC {}: {}", rpc_url, e))?;
    
    // Simple ERC20 balanceOf call using eth_call
    // balanceOf(address) = 0x70a08231
    let call_data = encode_balance_of_call(wallet_addr);
    
    let tx = TransactionRequest::new()
        .to(token_addr)
        .data(call_data);
    
    let result = provider
        .call(&tx.into(), None)
        .await
        .map_err(|e| format!("Failed to call balanceOf on {} for token {}: {}", chain_name, token_addr, e))?;
    
    // Decode the result as U256
    let balance = U256::from_big_endian(&result);
    Ok(balance.to_string())
}

/// Encode a balanceOf(address) call for ERC20 tokens
fn encode_balance_of_call(address: Address) -> Bytes {
    // balanceOf function selector (4 bytes): 0x70a08231
    // address parameter (32 bytes, padded)
    let mut data = vec![0x70, 0xa0, 0x82, 0x31];
    let mut addr_bytes = [0u8; 32];
    addr_bytes[12..].copy_from_slice(&address.to_fixed_bytes());
    data.extend_from_slice(&addr_bytes);
    Bytes::from(data)
}

/// Get all balances for a wallet on a specific chain
pub async fn get_chain_balances(
    chain_config: EvmChainConfig,
    wallet_address: &str,
) -> Result<Vec<(String, String, f64)>, String> {
    let mut balances = Vec::new();
    let assets = chain_config.assets();
    
    for asset in assets {
        // Add delay between requests to avoid rate limiting
        tokio::time::sleep(Duration::from_millis(REQUEST_DELAY_MS)).await;
        
        let balance_str = if asset.contract_address.is_none() {
            // Native token (ETH)
            query_eth_balance(chain_config, wallet_address).await?
        } else {
            // ERC20 token
            query_erc20_balance(
                chain_config,
                &asset.contract_address.clone().unwrap(),
                wallet_address,
            )
            .await?
        };
        
        // Convert balance string to float with decimals
        let balance_u256 = U256::from_dec_str(&balance_str)
            .unwrap_or(U256::zero());
        
        let divisor = U256::from(10).pow(U256::from(asset.decimals));
        let balance_float = if divisor > U256::zero() {
            let whole = balance_u256 / divisor;
            let remainder = balance_u256 % divisor;
            let remainder_f64 = remainder.as_u64() as f64 / divisor.as_u64() as f64;
            whole.as_u64() as f64 + remainder_f64
        } else {
            0.0
        };
        
        balances.push((asset.symbol.clone(), balance_str, balance_float));
    }
    
    Ok(balances)
}
