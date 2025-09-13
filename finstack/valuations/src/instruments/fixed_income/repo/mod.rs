//! Repurchase Agreement (Repo) instruments.
//!
//! This module provides functionality for pricing and risk management of repurchase agreements,
//! including collateral valuation, haircut calculations, and term structure modeling.

pub mod builder;
pub mod metrics;
mod types;

// Re-export main types
pub use types::*;

// Re-export builder
pub use builder::RepoBuilder;

// Private module for internal use
pub(crate) mod mod_repo {
    pub use super::builder::RepoBuilder;
}
