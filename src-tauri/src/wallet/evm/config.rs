use crate::wallet::types::EvmAsset;

/// Chain configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmChainConfig {
    Ethereum,
    Arbitrum,
    Optimism,
    Polygon,
    BinanceSmartChain,
}

impl EvmChainConfig {
    pub fn chain_id(&self) -> u64 {
        match self {
            Self::Ethereum => 1,
            Self::Arbitrum => 42161,
            Self::Optimism => 10,
            Self::Polygon => 137,
            Self::BinanceSmartChain => 56,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Arbitrum => "arbitrum",
            Self::Optimism => "optimism",
            Self::Polygon => "polygon",
            Self::BinanceSmartChain => "bsc",
        }
    }

    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        match self {
            Self::Ethereum => "Ethereum",
            Self::Arbitrum => "Arbitrum",
            Self::Optimism => "Optimism",
            Self::Polygon => "Polygon",
            Self::BinanceSmartChain => "Binance Smart Chain",
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
            Self::Optimism => {
                // Try to get from env var first, fall back to public RPC
                std::env::var("EVM_RPC_OPTIMISM_URL")
                    .unwrap_or_else(|_| {
                        // Fall back to free public RPC endpoint
                        "https://mainnet.optimism.io".to_string()
                    })
            },
            Self::Polygon => {
                // Try to get from env var first, fall back to public RPC
                std::env::var("EVM_RPC_POLYGON_URL")
                    .unwrap_or_else(|_| {
                        // Fall back to free public RPC endpoint
                        "https://polygon-rpc.com".to_string()
                    })
            },
            Self::BinanceSmartChain => {
                // Try to get from env var first, fall back to public RPC
                std::env::var("EVM_RPC_BSC_URL")
                    .unwrap_or_else(|_| {
                        // Fall back to free public RPC endpoint
                        "https://bsc-dataseed1.binance.org".to_string()
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
            Self::Optimism => vec![
                EvmAsset::new("ETH", "Ethereum", 18, None),
                EvmAsset::new("USDT", "Tether USD", 6, Some("0x94b008aA00579c1307B0EF2c499aD98a8ce58e58")),
                EvmAsset::new("USDC", "USD Coin", 6, Some("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85")),
            ],
            Self::Polygon => vec![
                EvmAsset::new("MATIC", "Polygon", 18, None),
                EvmAsset::new("USDT", "Tether USD", 6, Some("0xc2132D05D31c914a87C6611C10748AEb04B58e8F")),
                EvmAsset::new("USDC", "USD Coin", 6, Some("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359")),
            ],
            Self::BinanceSmartChain => vec![
                EvmAsset::new("BNB", "BNB", 18, None),
                EvmAsset::new("USDT", "Tether USD", 18, Some("0x55d398326f99059fF775485246999027B3197955")),
                EvmAsset::new("USDC", "USD Coin", 18, Some("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d")),
            ],
        }
    }
}

/// Get all supported chains
pub fn get_all_chains() -> Vec<EvmChainConfig> {
    vec![
        EvmChainConfig::Ethereum,
        EvmChainConfig::Arbitrum,
        EvmChainConfig::Optimism,
        EvmChainConfig::Polygon,
        EvmChainConfig::BinanceSmartChain,
    ]
}

/// Get chain by name
#[allow(dead_code)]
pub fn get_chain_by_name(name: &str) -> Option<EvmChainConfig> {
    match name {
        "ethereum" => Some(EvmChainConfig::Ethereum),
        "arbitrum" => Some(EvmChainConfig::Arbitrum),
        "optimism" => Some(EvmChainConfig::Optimism),
        "polygon" => Some(EvmChainConfig::Polygon),
        "bsc" => Some(EvmChainConfig::BinanceSmartChain),
        _ => None,
    }
}

/// Get chain by chain ID
#[allow(dead_code)]
pub fn get_chain_by_id(chain_id: u64) -> Option<EvmChainConfig> {
    match chain_id {
        1 => Some(EvmChainConfig::Ethereum),
        42161 => Some(EvmChainConfig::Arbitrum),
        10 => Some(EvmChainConfig::Optimism),
        137 => Some(EvmChainConfig::Polygon),
        56 => Some(EvmChainConfig::BinanceSmartChain),
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
        assert_eq!(chains.len(), 5);
        assert!(chains.contains(&EvmChainConfig::Ethereum));
        assert!(chains.contains(&EvmChainConfig::Arbitrum));
        assert!(chains.contains(&EvmChainConfig::Optimism));
        assert!(chains.contains(&EvmChainConfig::Polygon));
        assert!(chains.contains(&EvmChainConfig::BinanceSmartChain));
    }

    #[test]
    fn test_optimism_config() {
        let chain = EvmChainConfig::Optimism;
        assert_eq!(chain.chain_id(), 10);
        assert_eq!(chain.name(), "optimism");
        assert_eq!(chain.assets().len(), 3); // ETH, USDT, USDC
        assert_eq!(chain.assets()[0].symbol, "ETH");
    }

    #[test]
    fn test_polygon_config() {
        let chain = EvmChainConfig::Polygon;
        assert_eq!(chain.chain_id(), 137);
        assert_eq!(chain.name(), "polygon");
        assert_eq!(chain.assets().len(), 3); // MATIC, USDT, USDC
        assert_eq!(chain.assets()[0].symbol, "MATIC");
    }

    #[test]
    fn test_bsc_config() {
        let chain = EvmChainConfig::BinanceSmartChain;
        assert_eq!(chain.chain_id(), 56);
        assert_eq!(chain.name(), "bsc");
        assert_eq!(chain.assets().len(), 3); // BNB, USDT, USDC
        assert_eq!(chain.assets()[0].symbol, "BNB");
    }

    #[test]
    fn test_get_chain_by_name() {
        assert_eq!(get_chain_by_name("ethereum"), Some(EvmChainConfig::Ethereum));
        assert_eq!(get_chain_by_name("arbitrum"), Some(EvmChainConfig::Arbitrum));
        assert_eq!(get_chain_by_name("optimism"), Some(EvmChainConfig::Optimism));
        assert_eq!(get_chain_by_name("polygon"), Some(EvmChainConfig::Polygon));
        assert_eq!(get_chain_by_name("bsc"), Some(EvmChainConfig::BinanceSmartChain));
        assert_eq!(get_chain_by_name("unknown"), None);
    }

    #[test]
    fn test_get_chain_by_id() {
        assert_eq!(get_chain_by_id(1), Some(EvmChainConfig::Ethereum));
        assert_eq!(get_chain_by_id(42161), Some(EvmChainConfig::Arbitrum));
        assert_eq!(get_chain_by_id(10), Some(EvmChainConfig::Optimism));
        assert_eq!(get_chain_by_id(137), Some(EvmChainConfig::Polygon));
        assert_eq!(get_chain_by_id(56), Some(EvmChainConfig::BinanceSmartChain));
        assert_eq!(get_chain_by_id(999999), None);
    }
}
