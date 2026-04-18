//! Frozen public contracts for the wallet state model.
//!
//! Derived verbatim from Phase 2 Acceptance Criteria in the plan.
//! Fields and variants MUST match the plan code block byte-for-byte.
//! Gate 2 (scripts/check_task.sh) verifies this mechanically.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessStatus {
    Fresh,
    Cached,
    Stale,
    Unavailable,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessMetadata {
    pub status: FreshnessStatus,
    pub updated_at: Option<i64>,
    pub failed_sources: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceStatus {
    Fresh,
    Stale,
    Unavailable,
    Synthetic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceState {
    pub price_usd: Option<f64>,
    pub price_source: Option<String>,
    pub price_updated_at: Option<i64>,
    pub status: PriceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceState {
    pub raw_amount: String,
    pub display_amount: f64,
    pub chain_id: Option<String>,
    pub freshness: FreshnessMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioState {
    pub value_usd: Option<f64>,
    pub value_btc: Option<f64>,
    pub freshness: FreshnessMetadata,
}
