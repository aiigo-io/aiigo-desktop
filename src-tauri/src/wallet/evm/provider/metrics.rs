use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Runtime metrics for hybrid provider operations.
pub struct ProviderMetrics {
    wss_queries_total: AtomicU64,
    http_queries_total: AtomicU64,
    wss_failures: AtomicU64,
    http_failures: AtomicU64,
    wss_latency_sum_ms: AtomicU64,
    http_latency_sum_ms: AtomicU64,
}

impl ProviderMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            wss_queries_total: AtomicU64::new(0),
            http_queries_total: AtomicU64::new(0),
            wss_failures: AtomicU64::new(0),
            http_failures: AtomicU64::new(0),
            wss_latency_sum_ms: AtomicU64::new(0),
            http_latency_sum_ms: AtomicU64::new(0),
        })
    }

    pub fn record_wss_query(&self, latency: Duration, success: bool) {
        self.wss_queries_total.fetch_add(1, Ordering::Relaxed);
        self.wss_latency_sum_ms
            .fetch_add(latency.as_millis() as u64, Ordering::Relaxed);
        if !success {
            self.wss_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_http_query(&self, latency: Duration, success: bool) {
        self.http_queries_total.fetch_add(1, Ordering::Relaxed);
        self.http_latency_sum_ms
            .fetch_add(latency.as_millis() as u64, Ordering::Relaxed);
        if !success {
            self.http_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[allow(dead_code)]
    pub fn snapshot(&self) -> ProviderMetricsSnapshot {
        ProviderMetricsSnapshot {
            wss_queries_total: self.wss_queries_total.load(Ordering::Relaxed),
            http_queries_total: self.http_queries_total.load(Ordering::Relaxed),
            wss_failures: self.wss_failures.load(Ordering::Relaxed),
            http_failures: self.http_failures.load(Ordering::Relaxed),
            wss_latency_sum_ms: self.wss_latency_sum_ms.load(Ordering::Relaxed),
            http_latency_sum_ms: self.http_latency_sum_ms.load(Ordering::Relaxed),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProviderMetricsSnapshot {
    pub wss_queries_total: u64,
    pub http_queries_total: u64,
    pub wss_failures: u64,
    pub http_failures: u64,
    pub wss_latency_sum_ms: u64,
    pub http_latency_sum_ms: u64,
}

#[allow(dead_code)]
impl ProviderMetricsSnapshot {
    pub fn wss_avg_latency_ms(&self) -> u64 {
        if self.wss_queries_total == 0 {
            return 0;
        }
        self.wss_latency_sum_ms / self.wss_queries_total
    }

    pub fn http_avg_latency_ms(&self) -> u64 {
        if self.http_queries_total == 0 {
            return 0;
        }
        self.http_latency_sum_ms / self.http_queries_total
    }

    pub fn wss_success_rate(&self) -> f64 {
        success_rate(self.wss_queries_total, self.wss_failures)
    }

    pub fn http_success_rate(&self) -> f64 {
        success_rate(self.http_queries_total, self.http_failures)
    }
}

#[allow(dead_code)]
fn success_rate(total: u64, failures: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        ((total - failures) as f64 / total as f64) * 100.0
    }
}
