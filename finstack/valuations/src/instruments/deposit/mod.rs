//! Deposit instrument module.
//!
//! Provides the implementation of a simple money‑market deposit: principal is
//! exchanged at the start date and principal plus simple interest at maturity.
//! This module mirrors the structure used by other instruments (e.g., basis swap)
//! with clear separation between types, pricing engines, and metrics.

pub mod metrics;
pub mod pricing;
mod types;

pub use types::Deposit;
pub use pricing::engine::DepositEngine;

// Builder provided by FinancialBuilder derive
