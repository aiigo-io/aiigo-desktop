use crate::wallet::types::EvmAsset;

/// Chain configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmChainConfig {
    Ethereum,
    Arbitrum,
}

impl EvmChainConfig {
    pub fn chain_id(&self) -> u64 {
        match self {
            Self::Ethereum => 1,
            Self::Arbitrum => 42161,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Arbitrum => "arbitrum",
        }
    }

    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        match self {
            Self::Ethereum => "Ethereum",
            Self::Arbitrum => "Arbitrum",
        }
    }

    #[allow(dead_code)]
    pub fn rpc_url(&self) -> String {
        match self {
            Self::Ethereum => {
                // Try to get from env var first, fall back to public RPC
                std::env::var("ALCHEMY_ETHEREUM_URL")
                    .unwrap_or_else(|_| {
                        // Fall back to free public RPC endpoint
                        "https://eth.llamarpc.com".to_string()
                    })
            },
            Self::Arbitrum => {
                // Try to get from env var first, fall back to public RPC
                std::env::var("EVM_RPC_ARBITRUM_URL")
                    .unwrap_or_else(|_| {
                        // Fall back to free public RPC endpoint
                        "https://arb1.arbitrum.io/rpc".to_string()
                    })
            },
        }
    }

    pub fn assets(&self) -> Vec<EvmAsset> {
        match self {
            Self::Ethereum => vec![
                EvmAsset::new("ETH", "Ethereum", 18, None),
                EvmAsset::new("USDT", "Tether USD", 6, Some("0xdAC17F958D2ee523a2206206994597C13D831ec7")),
                EvmAsset::new("USDC", "USD Coin", 6, Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")),
            ],
            Self::Arbitrum => vec![
                EvmAsset::new("ETH", "Ethereum", 18, None),
                EvmAsset::new("USDT", "Tether USD", 6, Some("0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9")),
                EvmAsset::new("USDC", "USD Coin", 6, Some("0xaf88d065e77c8cC2239327C5EDb3A432268e5831")),
            ],
        }
    }
}

/// Get all supported chains
pub fn get_all_chains() -> Vec<EvmChainConfig> {
    vec![
        EvmChainConfig::Ethereum,
        EvmChainConfig::Arbitrum,
        // Add more chains here:
        // EvmChainConfig::Optimism,
        // EvmChainConfig::Polygon,
        // EvmChainConfig::BinanceSmartChain,
    ]
}

/// Get chain by name
#[allow(dead_code)]
pub fn get_chain_by_name(name: &str) -> Option<EvmChainConfig> {
    match name {
        "ethereum" => Some(EvmChainConfig::Ethereum),
        "arbitrum" => Some(EvmChainConfig::Arbitrum),
        _ => None,
    }
}

/// Get chain by chain ID
#[allow(dead_code)]
pub fn get_chain_by_id(chain_id: u64) -> Option<EvmChainConfig> {
    match chain_id {
        1 => Some(EvmChainConfig::Ethereum),
        42161 => Some(EvmChainConfig::Arbitrum),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethereum_config() {
        let chain = EvmChainConfig::Ethereum;
        assert_eq!(chain.chain_id(), 1);
        assert_eq!(chain.name(), "ethereum");
        assert_eq!(chain.assets().len(), 3);
    }

    #[test]
    fn test_arbitrum_config() {
        let chain = EvmChainConfig::Arbitrum;
        assert_eq!(chain.chain_id(), 42161);
        assert_eq!(chain.assets().len(), 3); // ETH, USDT, USDC
        assert_eq!(chain.assets()[0].symbol, "ETH");
    }

    #[test]
    fn test_get_all_chains() {
        let chains = get_all_chains();
        assert!(chains.len() >= 2);
    }

    #[test]
    fn test_get_chain_by_name() {
        assert_eq!(get_chain_by_name("ethereum"), Some(EvmChainConfig::Ethereum));
        assert_eq!(get_chain_by_name("arbitrum"), Some(EvmChainConfig::Arbitrum));
    }
}
