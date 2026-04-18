//! ChainAdapter trait.
//!
//! The method set is finalized during Phase 3 implementation. This skeleton
//! exists so sync engine code and transaction lifecycle code can type-depend
//! on the trait today, and so Gate 1 can see the module on disk.

#![allow(dead_code)]

/// Common interface that BTC (`wallet/bitcoin/*`) and EVM (`wallet/evm/*`)
/// wallet modules must implement. Phase 3 expands the method set to cover:
///   - native balance reads
///   - receipt / transaction status reads
///   - broadcast
///   - per-chain finality thresholds
///
/// Phase 3 may add an async variant; do not reorder existing methods.
pub trait ChainAdapter: Send + Sync {
    /// Short identifier for the chain family, e.g. "bitcoin" or "evm".
    fn chain_family(&self) -> &'static str;

    // Phase 3 method additions go below this line.
    // Keep `chain_family` first to preserve trait object layout expectations.
}
