//! Repurchase Agreement (Repo) instruments.
//!
//! This module provides functionality for pricing and risk management of repurchase agreements,
//! including collateral valuation, haircut calculations, and term structure modeling.

pub mod metrics;
mod types;

// Re-export main types
pub use types::*;

// Builder is generated via derive on `Repo`.
