use super::{HybridProvider, ProviderError};
use crate::wallet::evm::config::EvmChainConfig;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static PROVIDER_CACHE: Lazy<RwLock<HashMap<EvmChainConfig, Arc<HybridProvider>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Lazily builds and caches hybrid providers per chain.
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub async fn get_or_init(chain: EvmChainConfig) -> Result<Arc<HybridProvider>, ProviderError> {
        if let Some(provider) = Self::try_get(chain).await {
            return Ok(provider);
        }

        let config = chain.provider_config();
        if config.wss_enabled() {
            println!(
                "[PROVIDER] Bootstrapping {} provider (WSS enabled, pool={} timeout={}s)",
                chain.name(),
                config.pool_size,
                config.connect_timeout_secs
            );
        } else {
            println!(
                "[PROVIDER] Bootstrapping {} provider (HTTP only)",
                chain.name()
            );
        }

        let provider = Arc::new(HybridProvider::new(config, chain.name()).await?);

        let mut cache = PROVIDER_CACHE.write().await;
        Ok(cache
            .entry(chain)
            .or_insert_with(|| provider.clone())
            .clone())
    }

    async fn try_get(chain: EvmChainConfig) -> Option<Arc<HybridProvider>> {
        let cache = PROVIDER_CACHE.read().await;
        cache.get(&chain).cloned()
    }

    #[allow(dead_code)]
    pub async fn clear() {
        let mut cache = PROVIDER_CACHE.write().await;
        cache.clear();
    }
}
