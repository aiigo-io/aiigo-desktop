use crate::wallet::evm::provider::ProviderConfig;
use crate::wallet::types::EvmAsset;

fn env_with_fallback(keys: &[&str], fallback: &str) -> String {
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                return value;
            }
        }
    }
    fallback.to_string()
}

fn env_optional(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn env_bool(var: &str, default: bool) -> bool {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(default)
}

fn env_usize(var: &str, default: usize) -> usize {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_u64(var: &str, default: u64) -> u64 {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_u32(var: &str, default: u32) -> u32 {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(default)
}

/// Chain configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmChainConfig {
    Ethereum,
    Arbitrum,
    Optimism,
    Polygon,
    BinanceSmartChain,
    EthereumSepolia,
}

impl EvmChainConfig {
    pub fn chain_id(&self) -> u64 {
        match self {
            Self::Ethereum => 1,
            Self::Arbitrum => 42161,
            Self::Optimism => 10,
            Self::Polygon => 137,
            Self::BinanceSmartChain => 56,
            Self::EthereumSepolia => 11155111,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Arbitrum => "arbitrum",
            Self::Optimism => "optimism",
            Self::Polygon => "polygon",
            Self::BinanceSmartChain => "bsc",
            Self::EthereumSepolia => "sepolia",
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
            Self::EthereumSepolia => "Ethereum Sepolia",
        }
    }

    #[allow(dead_code)]
    pub fn rpc_url(&self) -> String {
        match self {
            Self::Ethereum => env_with_fallback(
                &["ETHEREUM_HTTP_URL", "ALCHEMY_ETHEREUM_URL"],
                "https://eth.llamarpc.com",
            ),
            Self::Arbitrum => env_with_fallback(
                &["ARBITRUM_HTTP_URL", "EVM_RPC_ARBITRUM_URL"],
                "https://arb1.arbitrum.io/rpc",
            ),
            Self::Optimism => env_with_fallback(
                &["OPTIMISM_HTTP_URL", "EVM_RPC_OPTIMISM_URL"],
                "https://mainnet.optimism.io",
            ),
            Self::Polygon => env_with_fallback(
                &["POLYGON_HTTP_URL", "EVM_RPC_POLYGON_URL"],
                "https://polygon-rpc.com",
            ),
            Self::BinanceSmartChain => env_with_fallback(
                &["BSC_HTTP_URL", "EVM_RPC_BSC_URL"],
                "https://bsc-dataseed1.binance.org",
            ),
            Self::EthereumSepolia => env_with_fallback(
                &["ETHEREUM_SEPOLIA_HTTP_URL", "EVM_RPC_SEPOLIA_URL"],
                "https://ethereum-sepolia-rpc.publicnode.com",
            ),
        }
    }

    pub fn wss_url(&self) -> Option<String> {
        match self {
            Self::Ethereum => env_optional("ETHEREUM_WSS_URL"),
            Self::Arbitrum => env_optional("ARBITRUM_WSS_URL"),
            Self::Optimism => env_optional("OPTIMISM_WSS_URL"),
            Self::Polygon => env_optional("POLYGON_WSS_URL"),
            Self::BinanceSmartChain => env_optional("BSC_WSS_URL"),
            Self::EthereumSepolia => env_optional("ETHEREUM_SEPOLIA_WSS_URL"),
        }
    }

    pub fn provider_config(&self) -> ProviderConfig {
        ProviderConfig {
            http_url: self.rpc_url(),
            wss_url: self.wss_url(),
            enable_wss: env_bool("ENABLE_WSS", true),
            pool_size: env_usize("WSS_POOL_SIZE", 2),
            connect_timeout_secs: env_u64("WSS_CONNECT_TIMEOUT", 10),
            health_check_interval_secs: env_u64("WSS_HEALTH_CHECK_INTERVAL", 30),
            auto_reconnect: env_bool("WSS_AUTO_RECONNECT", true),
            max_reconnect_attempts: env_u32("WSS_MAX_RECONNECT_ATTEMPTS", 3),
        }
    }

    pub fn assets(&self) -> Vec<EvmAsset> {
        match self {
            Self::Ethereum => vec![
                EvmAsset::new("ETH", "Ethereum", 18, None),
                EvmAsset::new(
                    "USDT",
                    "Tether USD",
                    6,
                    Some("0xdAC17F958D2ee523a2206206994597C13D831ec7"),
                ),
                EvmAsset::new(
                    "USDC",
                    "USD Coin",
                    6,
                    Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
                ),
            ],
            Self::Arbitrum => vec![
                EvmAsset::new("ETH", "Ethereum", 18, None),
                EvmAsset::new(
                    "USDT",
                    "Tether USD",
                    6,
                    Some("0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9"),
                ),
                EvmAsset::new(
                    "USDC",
                    "USD Coin",
                    6,
                    Some("0xaf88d065e77c8cC2239327C5EDb3A432268e5831"),
                ),
            ],
            Self::Optimism => vec![
                EvmAsset::new("ETH", "Ethereum", 18, None),
                EvmAsset::new(
                    "USDT",
                    "Tether USD",
                    6,
                    Some("0x94b008aA00579c1307B0EF2c499aD98a8ce58e58"),
                ),
                EvmAsset::new(
                    "USDC",
                    "USD Coin",
                    6,
                    Some("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85"),
                ),
            ],
            Self::Polygon => vec![
                EvmAsset::new("MATIC", "Polygon", 18, None),
                EvmAsset::new(
                    "USDT",
                    "Tether USD",
                    6,
                    Some("0xc2132D05D31c914a87C6611C10748AEb04B58e8F"),
                ),
                EvmAsset::new(
                    "USDC",
                    "USD Coin",
                    6,
                    Some("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"),
                ),
            ],
            Self::BinanceSmartChain => vec![
                EvmAsset::new("BNB", "BNB", 18, None),
                EvmAsset::new(
                    "USDT",
                    "Tether USD",
                    18,
                    Some("0x55d398326f99059fF775485246999027B3197955"),
                ),
                EvmAsset::new(
                    "USDC",
                    "USD Coin",
                    18,
                    Some("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d"),
                ),
            ],
            Self::EthereumSepolia => vec![
                EvmAsset::new("ETH", "Ethereum", 18, None),
                EvmAsset::new(
                    "USDC",
                    "USD Coin",
                    6,
                    Some("0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238"),
                ),
                EvmAsset::new(
                    "USDT",
                    "Tether USD",
                    6,
                    Some("0xE50d86c6dE38F9754f6777d2925377564Bf79482"),
                ),
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
        EvmChainConfig::EthereumSepolia,
    ]
}

/// Max number of chains queried concurrently. Defaults to 3.
pub fn chain_concurrency_limit() -> usize {
    env_usize("EVM_CHAIN_CONCURRENCY", 3)
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
        "sepolia" => Some(EvmChainConfig::EthereumSepolia),
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
        11155111 => Some(EvmChainConfig::EthereumSepolia),
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
    fn test_ethereum_sepolia_config() {
        let chain = EvmChainConfig::EthereumSepolia;
        assert_eq!(chain.chain_id(), 11155111);
        assert_eq!(chain.name(), "sepolia");
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
        assert_eq!(chains.len(), 6);
        assert!(chains.contains(&EvmChainConfig::Ethereum));
        assert!(chains.contains(&EvmChainConfig::Arbitrum));
        assert!(chains.contains(&EvmChainConfig::Optimism));
        assert!(chains.contains(&EvmChainConfig::Polygon));
        assert!(chains.contains(&EvmChainConfig::BinanceSmartChain));
        assert!(chains.contains(&EvmChainConfig::EthereumSepolia));
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
        assert_eq!(
            get_chain_by_name("ethereum"),
            Some(EvmChainConfig::Ethereum)
        );
        assert_eq!(
            get_chain_by_name("arbitrum"),
            Some(EvmChainConfig::Arbitrum)
        );
        assert_eq!(
            get_chain_by_name("optimism"),
            Some(EvmChainConfig::Optimism)
        );
        assert_eq!(get_chain_by_name("polygon"), Some(EvmChainConfig::Polygon));
        assert_eq!(
            get_chain_by_name("bsc"),
            Some(EvmChainConfig::BinanceSmartChain)
        );
        assert_eq!(
            get_chain_by_name("sepolia"),
            Some(EvmChainConfig::EthereumSepolia)
        );
        assert_eq!(get_chain_by_name("unknown"), None);
    }

    #[test]
    fn test_get_chain_by_id() {
        assert_eq!(get_chain_by_id(1), Some(EvmChainConfig::Ethereum));
        assert_eq!(get_chain_by_id(42161), Some(EvmChainConfig::Arbitrum));
        assert_eq!(get_chain_by_id(10), Some(EvmChainConfig::Optimism));
        assert_eq!(get_chain_by_id(137), Some(EvmChainConfig::Polygon));
        assert_eq!(get_chain_by_id(56), Some(EvmChainConfig::BinanceSmartChain));
        assert_eq!(get_chain_by_id(11155111), Some(EvmChainConfig::EthereumSepolia));
        assert_eq!(get_chain_by_id(999999), None);
    }
}
