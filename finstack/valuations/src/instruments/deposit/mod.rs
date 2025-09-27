//! Deposit instrument module.
//!
//! Provides the implementation of a simple money‑market deposit: principal is
//! exchanged at the start date and principal plus simple interest at maturity.
//! This module mirrors the structure used by other instruments with clear
//! separation between types, pricing implementation, and metrics.

pub mod metrics;
pub mod pricer;
mod types;

pub use types::Deposit;

// Builder provided by FinancialBuilder derive
