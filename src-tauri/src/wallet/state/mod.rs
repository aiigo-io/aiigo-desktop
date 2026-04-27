//! Wallet state model subsystem.
//! This module exposes the current wallet MVP state layer.
//! Reference: docs/architecture/executable-wallet-runtime-blueprint.md

#![allow(dead_code)]

pub mod commands;
pub mod freshness;
pub mod portfolio;
pub mod price;
pub mod types;
