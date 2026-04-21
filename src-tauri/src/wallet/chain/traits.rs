//! ChainAdapter trait.
//!
//! This trait defines the current normalized chain boundary used by the MVP
//! sync engine and transaction flows.

#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq)]
pub struct ChainAssetBalanceSnapshot {
    pub symbol: String,
    pub name: String,
    pub contract_address: Option<String>,
    pub raw_amount: String,
    pub display_amount: f64,
    pub decimals: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChainBalanceSnapshot {
    pub chain_family: &'static str,
    pub chain_name: String,
    pub chain_id: Option<String>,
    pub wallet_address: String,
    pub assets: Vec<ChainAssetBalanceSnapshot>,
}

impl ChainBalanceSnapshot {
    pub fn total_display_amount(&self) -> f64 {
        self.assets.iter().map(|asset| asset.display_amount).sum()
    }
}

/// Common interface that BTC (`wallet/bitcoin/*`) and EVM (`wallet/evm/*`)
/// wallet modules must implement. Future extensions may add support for:
///   - native balance reads
///   - receipt / transaction status reads
///   - broadcast
///   - per-chain finality thresholds
pub trait ChainAdapter: Send + Sync {
    /// Short identifier for the chain family, e.g. "bitcoin" or "evm".
    fn chain_family(&self) -> &'static str;

    /// Stable chain identifier used by sync routing and persistence wiring.
    fn chain_name(&self) -> &str;

    /// Chain-specific ID where available. BTC intentionally returns None.
    fn chain_id(&self) -> Option<String>;

    /// Wallet address currently targeted by this adapter.
    fn wallet_address(&self) -> &str;

    /// Fetches a normalized balance snapshot while allowing chain-specific
    /// asset structure to survive in the returned asset list.
    fn fetch_balances<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<ChainBalanceSnapshot, String>> + Send + 'a>>;

    // Add future methods below this line while preserving existing callers.
}

#[cfg(test)]
mod tests {
    use super::{ChainAssetBalanceSnapshot, ChainBalanceSnapshot};

    #[test]
    fn balance_snapshot_total_sums_assets() {
        let snapshot = ChainBalanceSnapshot {
            chain_family: "evm",
            chain_name: "ethereum".to_string(),
            chain_id: Some("1".to_string()),
            wallet_address: "0xabc".to_string(),
            assets: vec![
                ChainAssetBalanceSnapshot {
                    symbol: "ETH".to_string(),
                    name: "Ethereum".to_string(),
                    contract_address: None,
                    raw_amount: "1000000000000000000".to_string(),
                    display_amount: 1.0,
                    decimals: 18,
                },
                ChainAssetBalanceSnapshot {
                    symbol: "USDC".to_string(),
                    name: "USD Coin".to_string(),
                    contract_address: Some("0xa0b8".to_string()),
                    raw_amount: "2500000".to_string(),
                    display_amount: 2.5,
                    decimals: 6,
                },
            ],
        };

        assert_eq!(snapshot.total_display_amount(), 3.5);
    }
}
