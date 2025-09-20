//! Repurchase Agreement (Repo) instruments.
//!
//! Follows the standard instrument layout used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing facade and engine implementation
//! - `metrics`: metric calculators and registry hook
//!
//! This module provides functionality for pricing and risk management of
//! repurchase agreements, including collateral valuation, haircut calculations,
//! and term structure modeling.

pub mod metrics;
pub mod pricing;
mod types;

// Re-export main types
pub use types::*;

// Builder is generated via derive on `Repo`.
