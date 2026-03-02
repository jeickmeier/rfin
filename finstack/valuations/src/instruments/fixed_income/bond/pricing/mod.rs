//! Bond pricing engines, registry pricers, and utilities.
//!
//! # Engines (`engine/`)
//!
//! Core pricing math for each model:
//! - **Discount**: PV = sum(CF_i * DF_i) using discount curves
//! - **Hazard**: Survival-weighted PV + fractional recovery of par (FRP)
//! - **Tree**: Binomial tree for callable/putable bonds and OAS
//! - **Merton MC**: Structural credit Monte Carlo for PIK bonds (feature-gated)
//!
//! # Pricers (`pricer/`)
//!
//! Thin registry adapters that downcast instruments, call engines, and return
//! `ValuationResult` for the pricer registry.
//!
//! # Utilities
//!
//! - `quote_conversions`: Price/yield/spread conversion functions
//! - `ytm_solver`: Robust yield-to-maturity calculation
//! - `settlement`: Settlement date and accrued interest utilities

pub mod engine;
pub(crate) mod pricer;
pub mod quote_conversions;
pub(crate) mod settlement;
pub mod ytm_solver;

// Backward-compatible re-exports so existing `use ...::discount_engine::BondEngine`
// paths continue to work.
pub use engine::discount as discount_engine;
pub use engine::hazard as hazard_engine;
#[cfg(feature = "mc")]
pub use engine::merton_mc as merton_mc_engine;
pub use engine::tree as tree_engine;
pub use quote_conversions as quote_engine;
