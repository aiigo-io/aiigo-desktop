use crate::wallet::state::types::FreshnessMetadata;
use crate::wallet::sync::types::SyncOutcome;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub id: String,
    pub label: String,
    pub wallet_type: String, // "mnemonic" or "private-key"
    pub address: String,
    pub balance: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletResponse {
    pub revealed_secret: Option<String>,
    pub revealed_secret_type: Option<String>,
    pub wallet: WalletInfo,
}

impl CreateWalletResponse {
    pub fn with_revealed_secret(wallet: WalletInfo, secret: String, secret_type: &str) -> Self {
        Self {
            revealed_secret: Some(secret),
            revealed_secret_type: Some(secret_type.to_string()),
            wallet,
        }
    }

    pub fn without_revealed_secret(wallet: WalletInfo) -> Self {
        Self {
            revealed_secret: None,
            revealed_secret_type: None,
            wallet,
        }
    }
}

// EVM Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EvmChain {
    Ethereum,
    Arbitrum,
}

impl EvmChain {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            EvmChain::Ethereum => "ethereum",
            EvmChain::Arbitrum => "arbitrum",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ethereum" => Some(EvmChain::Ethereum),
            "arbitrum" => Some(EvmChain::Arbitrum),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn chain_id(&self) -> u64 {
        match self {
            EvmChain::Ethereum => 1,
            EvmChain::Arbitrum => 42161,
        }
    }

    #[allow(dead_code)]
    pub fn rpc_url(&self) -> &str {
        match self {
            EvmChain::Ethereum => "https://eth.llamarpc.com",
            EvmChain::Arbitrum => "https://arbitrum.llamarpc.com",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmAsset {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub contract_address: Option<String>, // None for native asset (ETH)
}

impl EvmAsset {
    /// Create a new asset with detailed configuration
    pub fn new(symbol: &str, name: &str, decimals: u8, contract_address: Option<&str>) -> Self {
        EvmAsset {
            symbol: symbol.to_string(),
            name: name.to_string(),
            decimals,
            contract_address: contract_address.map(|s| s.to_string()),
        }
    }

    pub fn eth() -> Self {
        Self::new("ETH", "Ethereum", 18, None)
    }

    pub fn usdt() -> Self {
        Self::new(
            "USDT",
            "Tether USD",
            6,
            Some("0xdAC17F958D2ee523a2206206994597C13D831ec7"),
        )
    }

    pub fn usdc() -> Self {
        Self::new(
            "USDC",
            "USD Coin",
            6,
            Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        )
    }

    /// Get assets for a specific chain (deprecated - use config module instead)
    /// This is kept for backward compatibility
    #[allow(dead_code)]
    pub fn get_assets_for_chain(chain: &EvmChain) -> Vec<EvmAsset> {
        match chain {
            EvmChain::Ethereum => vec![EvmAsset::eth(), EvmAsset::usdt(), EvmAsset::usdc()],
            EvmChain::Arbitrum => vec![
                Self::new(
                    "USDT",
                    "Tether USD",
                    6,
                    Some("0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9"),
                ),
                Self::new(
                    "USDC",
                    "USD Coin",
                    6,
                    Some("0xff970a61a04b1ca14834a43f5de4533ebddb5cc8"),
                ),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmAssetBalance {
    pub chain: String,
    pub asset: EvmAsset,
    pub balance: String,    // Store as string to preserve precision
    pub balance_float: f64, // For UI display
    pub usd_price: f64,     // Current USD price
    pub usd_value: f64,     // Total value in USD (balance_float * usd_price)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmChainAssets {
    pub chain: String,
    pub chain_id: u64,
    pub total_balance_usd: f64,
    pub freshness: FreshnessMetadata,
    pub assets: Vec<EvmAssetBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmWalletInfo {
    pub id: String,
    pub label: String,
    pub wallet_type: String, // "mnemonic" or "private-key"
    pub address: String,
    pub chains: Vec<EvmChainAssets>,
    pub total_balance_usd: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmWalletBalancesResponse {
    pub wallet: EvmWalletInfo,
    pub sync: SyncOutcome,
}

#[cfg(test)]
mod tests {
    use super::{
        EvmAsset, EvmAssetBalance, EvmChainAssets, EvmWalletBalancesResponse, EvmWalletInfo,
    };
    use crate::wallet::state::types::{FreshnessMetadata, FreshnessStatus};
    use crate::wallet::sync::types::{SyncOutcome, SyncReason, SyncTarget};
    use serde_json::Value;

    fn top_level_keys(value: &Value) -> Vec<String> {
        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    #[test]
    fn evm_wallet_balances_response_serializes_wallet_sync_and_chain_freshness() {
        let response = EvmWalletBalancesResponse {
            wallet: EvmWalletInfo {
                id: "wallet-1".to_string(),
                label: "Main".to_string(),
                wallet_type: "mnemonic".to_string(),
                address: "0xabc".to_string(),
                chains: vec![EvmChainAssets {
                    chain: "ethereum".to_string(),
                    chain_id: 1,
                    total_balance_usd: 12.5,
                    freshness: FreshnessMetadata {
                        status: FreshnessStatus::Stale,
                        updated_at: None,
                        failed_sources: vec!["ethereum".to_string()],
                    },
                    assets: vec![EvmAssetBalance {
                        chain: "ethereum".to_string(),
                        asset: EvmAsset::new("ETH", "Ethereum", 18, None),
                        balance: "1000000000000000000".to_string(),
                        balance_float: 1.0,
                        usd_price: 12.5,
                        usd_value: 12.5,
                    }],
                }],
                total_balance_usd: 12.5,
                created_at: "2026-04-20T00:00:00Z".to_string(),
                updated_at: "2026-04-20T00:00:00Z".to_string(),
            },
            sync: SyncOutcome {
                reason: SyncReason::Manual,
                target: SyncTarget::EvmWalletBalances,
                updated_at: 1_713_571_200,
                partial: true,
                failed_sources: vec!["ethereum".to_string()],
            },
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(top_level_keys(&json), vec!["sync", "wallet"]);

        let chain = &json["wallet"]["chains"][0];
        assert_eq!(
            top_level_keys(chain),
            vec!["assets", "chain", "chain_id", "freshness", "total_balance_usd"]
        );
        assert_eq!(chain["freshness"]["status"], "stale");
        assert_eq!(json["sync"]["partial"], true);
        assert_eq!(json["sync"]["failed_sources"][0], "ethereum");
    }
}
