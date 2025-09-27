//! Repurchase Agreement (Repo) instruments.
//!
//! Simplified instrument layout after refactoring:
//! - `types`: instrument data structures with integrated pricing logic
//! - `pricer`: registry integration using generic pricer
//! - `metrics`: metric calculators and registry hook
//!
//! This module provides functionality for pricing and risk management of
//! repurchase agreements, including collateral valuation, haircut calculations,
//! and term structure modeling.

pub mod metrics;
pub mod pricer;
mod types;

// Re-export main types
pub use types::*;

// Builder is generated via derive on `Repo`.
