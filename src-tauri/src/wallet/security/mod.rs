//! Security boundary subsystem.
//! This module owns the signer-session and authorized secret-read boundary
//! used by the current wallet MVP.
//! Reference: docs/architecture/executable-wallet-runtime-blueprint.md

#![allow(dead_code)]

pub mod types;
pub mod keystore;
pub mod backend;
pub mod session;
pub mod log_sanitize;
pub mod commands;
pub mod secret_envelope;
#[allow(unused_imports)]
pub use crate::safe_log;
#[allow(unused_imports)]
pub use log_sanitize::sanitize;
