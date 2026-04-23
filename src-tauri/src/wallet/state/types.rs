//! Frozen public contracts for the wallet state model.
//!
//! These are the current wallet MVP state contracts.
//! Keep them stable enough for the active UI surfaces, but do not treat this
//! file as a mechanically frozen plan artifact.
//! Reference: docs/architecture/executable-wallet-runtime-blueprint.md

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
    Cached,
    Stale,
    Partial,
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
