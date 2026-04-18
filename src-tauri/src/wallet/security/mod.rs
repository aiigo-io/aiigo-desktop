//! Security boundary subsystem.
//! Filled in Phase 1 of the wallet foundation hardening plan.
//! Plan reference: docs/superpowers/plans/2026-04-18-wallet-foundation-hardening.md

#![allow(dead_code)]

pub mod types;
pub mod keystore;
pub mod session;
// pub mod log_sanitize; // Phase 1
// pub mod commands;     // Phase 1
