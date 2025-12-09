use super::types::{ProviderConfig, ProviderError};
use ethers::providers::{Provider, Ws};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

pub struct WssConnectionPool {
    chain_name: String,
    config: ProviderConfig,
    connections: Arc<Mutex<Vec<Arc<Provider<Ws>>>>>,
    current_index: Mutex<usize>,
}

impl WssConnectionPool {
    pub async fn new(
        config: ProviderConfig,
        chain_name: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        let chain_name = chain_name.into();
        let wss_url = config.wss_url.clone().ok_or_else(|| {
            ProviderError::WssConnectionFailed(format!("{} WSS URL not configured", chain_name))
        })?;

        let pool_size = config.pool_size.max(1);
        let mut connections = Vec::new();

        for i in 0..pool_size {
            match Self::connect(&wss_url, config.connect_timeout_secs).await {
                Ok(provider) => {
                    tracing::info!(chain=%chain_name, connection=%(i + 1), pool_size=%pool_size, "Established WSS connection");
                    connections.push(provider);
                }
                Err(err) => {
                    tracing::warn!(chain=%chain_name, connection=%(i + 1), pool_size=%pool_size, error=%err.to_string(), "Failed to establish WSS connection");
                    if connections.is_empty() {
                        return Err(err);
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(Self {
            chain_name,
            config,
            connections: Arc::new(Mutex::new(connections)),
            current_index: Mutex::new(0),
        })
    }

    async fn connect(
        wss_url: &str,
        connect_timeout_secs: u64,
    ) -> Result<Arc<Provider<Ws>>, ProviderError> {
        let timeout_secs = connect_timeout_secs.max(1);
        let timeout_duration = Duration::from_secs(timeout_secs);

        let provider = timeout(timeout_duration, Provider::<Ws>::connect(wss_url))
            .await
            .map_err(|_| {
                ProviderError::WssConnectionFailed(format!(
                    "Timed out connecting to {} after {}s",
                    wss_url, timeout_secs
                ))
            })?
            .map_err(|e| ProviderError::WssConnectionFailed(format!("{}", e)))?;

        Ok(Arc::new(provider))
    }

    pub async fn acquire(&self) -> Result<Arc<Provider<Ws>>, ProviderError> {
        let connections = self.connections.lock().await;
        if connections.is_empty() {
            return Err(ProviderError::WssConnectionFailed(format!(
                "{} has no healthy WSS connections",
                self.chain_name
            )));
        }

        let mut index = self.current_index.lock().await;
        let provider = connections[*index].clone();
        *index = (*index + 1) % connections.len();
        Ok(provider)
    }

    #[allow(dead_code)]
    pub async fn size(&self) -> usize {
        self.connections.lock().await.len()
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }
}
