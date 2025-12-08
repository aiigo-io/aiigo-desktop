use super::pool::WssConnectionPool;
use super::types::{ProviderConfig, ProviderError};
use ethers::providers::Middleware;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Duration};

/// Background task that ensures WSS pools stay healthy and reconnect automatically.
pub struct HealthMonitor {
    chain_name: String,
    config: ProviderConfig,
    pool_handle: Arc<RwLock<Option<Arc<WssConnectionPool>>>>,
}

impl HealthMonitor {
    pub fn spawn(
        chain_name: String,
        config: ProviderConfig,
        pool_handle: Arc<RwLock<Option<Arc<WssConnectionPool>>>>,
    ) {
        let monitor = Arc::new(Self {
            chain_name,
            config,
            pool_handle,
        });

        tokio::spawn(async move {
            monitor.run().await;
        });
    }

    async fn run(self: Arc<Self>) {
        let interval_secs = self.config.health_check_interval_secs.max(5);
        let mut ticker = interval(Duration::from_secs(interval_secs));

        loop {
            ticker.tick().await;

            let needs_reconnect = match self.check_once().await {
                Ok(_) => false,
                Err(err) => {
                    eprintln!("[HEALTH][{}] Health check failed: {}", self.chain_name, err);
                    true
                }
            };

            if needs_reconnect {
                if let Err(err) = self.reconnect().await {
                    eprintln!(
                        "[HEALTH][{}] Reconnect attempts exhausted: {}",
                        self.chain_name, err
                    );
                }
            }
        }
    }

    async fn check_once(&self) -> Result<(), ProviderError> {
        let pool = { self.pool_handle.read().await.clone() };
        let pool = pool.ok_or_else(|| {
            ProviderError::WssConnectionFailed(format!(
                "{} has no active WSS connections",
                self.chain_name
            ))
        })?;

        let provider = pool.acquire().await?;
        provider
            .get_block_number()
            .await
            .map(|_| ())
            .map_err(|err| ProviderError::WssConnectionFailed(format!("{}", err)))
    }

    async fn reconnect(&self) -> Result<(), ProviderError> {
        if !self.config.auto_reconnect || !self.config.wss_enabled() {
            return Err(ProviderError::WssConnectionFailed(format!(
                "[{}] Auto reconnect disabled",
                self.chain_name
            )));
        }

        let attempts = self.config.max_reconnect_attempts.max(1);
        for attempt in 1..=attempts {
            let exponent = (attempt - 1).min(10);
            let backoff_ms = 1_000 * 2_u64.pow(exponent);
            println!(
                "[HEALTH][{}] Reconnect attempt {}/{} (waiting {}ms)",
                self.chain_name, attempt, attempts, backoff_ms
            );

            sleep(Duration::from_millis(backoff_ms)).await;

            match WssConnectionPool::new(self.config.clone(), self.chain_name.clone()).await {
                Ok(pool) => {
                    let mut guard = self.pool_handle.write().await;
                    *guard = Some(Arc::new(pool));
                    println!(
                        "[HEALTH][{}] Reconnected WSS pool on attempt {}/{}",
                        self.chain_name, attempt, attempts
                    );
                    return Ok(());
                }
                Err(err) => {
                    eprintln!(
                        "[HEALTH][{}] Reconnect attempt {}/{} failed: {}",
                        self.chain_name, attempt, attempts, err
                    );
                }
            }
        }

        Err(ProviderError::WssConnectionFailed(format!(
            "Failed to reconnect {} after {} attempts",
            self.chain_name, attempts
        )))
    }
}
