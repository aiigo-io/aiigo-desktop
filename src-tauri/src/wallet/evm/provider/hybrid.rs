use super::health::HealthMonitor;
use super::metrics::ProviderMetrics;
use super::pool::WssConnectionPool;
use super::types::{ProviderConfig, ProviderError};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, Bytes, U256};
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct HybridProvider {
    http_provider: Provider<Http>,
    ws_pool: Arc<RwLock<Option<Arc<WssConnectionPool>>>>,
    metrics: Arc<ProviderMetrics>,
    chain_name: String,
}

impl HybridProvider {
    pub async fn new(
        config: ProviderConfig,
        chain_name: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        let chain_name = chain_name.into();
        let http_provider = Provider::<Http>::try_from(config.http_url.as_str())
            .map_err(|e| ProviderError::HttpConnectionFailed(format!("{}", e)))?;

        let metrics = ProviderMetrics::new();
        let ws_pool = Arc::new(RwLock::new(None));

        if config.wss_enabled() {
            match WssConnectionPool::new(config.clone(), chain_name.clone()).await {
                Ok(pool) => {
                    let mut guard = ws_pool.write().await;
                    *guard = Some(Arc::new(pool));
                }
                Err(err) => {
                    tracing::warn!(chain=%chain_name, error=%err.to_string(), "WSS pool build failed, falling back to HTTP");
                }
            }

            if config.auto_reconnect {
                HealthMonitor::spawn(chain_name.clone(), config.clone(), ws_pool.clone());
            }
        }

        Ok(Self {
            http_provider,
            ws_pool,
            metrics,
            chain_name,
        })
    }

    #[allow(dead_code)]
    pub fn metrics(&self) -> Arc<ProviderMetrics> {
        self.metrics.clone()
    }

    async fn current_ws_pool(&self) -> Option<Arc<WssConnectionPool>> {
        let guard = self.ws_pool.read().await;
        guard.clone()
    }

    pub async fn get_balance(&self, address: Address) -> Result<U256, ProviderError> {
        if let Some(pool) = self.current_ws_pool().await {
            if let Ok(provider) = pool.acquire().await {
                let start = Instant::now();
                match provider.get_balance(address, None).await {
                    Ok(balance) => {
                        self.metrics.record_wss_query(start.elapsed(), true);
                        tracing::info!(chain=%self.chain_name, elapsed_ms=%start.elapsed().as_millis(), "WSS balance query succeeded");
                        return Ok(balance);
                    }
                    Err(err) => {
                        self.metrics.record_wss_query(start.elapsed(), false);
                        tracing::warn!(chain=%self.chain_name, error=%err.to_string(), "WSS balance query failed, falling back to HTTP");
                    }
                }
            }
        }

        let start = Instant::now();
        self.http_provider
            .get_balance(address, None)
            .await
            .map(|balance| {
                self.metrics.record_http_query(start.elapsed(), true);
                tracing::info!(chain=%self.chain_name, elapsed_ms=%start.elapsed().as_millis(), "HTTP balance query succeeded");
                balance
            })
            .map_err(|e| {
                self.metrics.record_http_query(start.elapsed(), false);
                ProviderError::AllProvidersFailed(format!(
                    "[{}] HTTP balance query failed: {}",
                    self.chain_name, e
                ))
            })
    }

    pub async fn call_contract(&self, tx: &TypedTransaction) -> Result<Bytes, ProviderError> {
        if let Some(pool) = self.current_ws_pool().await {
            if let Ok(provider) = pool.acquire().await {
                let start = Instant::now();
                match provider.call(tx, None).await {
                    Ok(result) => {
                        self.metrics.record_wss_query(start.elapsed(), true);
                        tracing::info!(chain=%self.chain_name, elapsed_ms=%start.elapsed().as_millis(), "WSS contract call succeeded");
                        return Ok(result);
                    }
                    Err(err) => {
                        self.metrics.record_wss_query(start.elapsed(), false);
                        tracing::warn!(chain=%self.chain_name, error=%err.to_string(), "WSS contract call failed, falling back to HTTP");
                    }
                }
            }
        }

        let start = Instant::now();
        self.http_provider
            .call(tx, None)
            .await
            .map(|result| {
                self.metrics.record_http_query(start.elapsed(), true);
                tracing::info!(chain=%self.chain_name, elapsed_ms=%start.elapsed().as_millis(), "HTTP contract call succeeded");
                result
            })
            .map_err(|e| {
                self.metrics.record_http_query(start.elapsed(), false);
                ProviderError::AllProvidersFailed(format!(
                    "[{}] HTTP contract call failed: {}",
                    self.chain_name, e
                ))
            })
    }
}
