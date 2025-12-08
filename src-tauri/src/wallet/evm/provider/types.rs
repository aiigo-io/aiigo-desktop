use thiserror::Error;

/// Configuration options for creating a hybrid HTTP + WSS provider.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub http_url: String,
    pub wss_url: Option<String>,
    pub enable_wss: bool,
    pub pool_size: usize,
    pub connect_timeout_secs: u64,
    pub health_check_interval_secs: u64,
    pub auto_reconnect: bool,
    pub max_reconnect_attempts: u32,
}

impl ProviderConfig {
    pub fn wss_enabled(&self) -> bool {
        self.enable_wss && self.wss_url.is_some()
    }
}

/// Error variants emitted by provider operations.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("WSS connection failed: {0}")]
    WssConnectionFailed(String),
    #[error("HTTP connection failed: {0}")]
    HttpConnectionFailed(String),
    #[error("All providers failed: {0}")]
    AllProvidersFailed(String),
}
