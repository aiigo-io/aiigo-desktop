pub mod bitcoin;
pub mod evm;
pub mod types;
pub mod transaction_types;
pub mod transaction_commands;

// Hardening-plan subsystems. Bodies filled in Phase 1-3.
// Plan reference: docs/superpowers/plans/2026-04-18-wallet-foundation-hardening.md
pub mod security;
pub mod state;
pub mod sync;
pub mod chain;
