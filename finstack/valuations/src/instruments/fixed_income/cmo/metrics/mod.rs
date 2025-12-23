//! Agency CMO risk metrics.
//!
//! CMO-specific metrics including tranche-level OAS, duration,
//! and scenario analysis.

pub mod oas;

pub use oas::calculate_tranche_oas;
