//! Copula models for CDS tranche pricing.
//!
//! This module re-exports the shared copula infrastructure from
//! [`crate::instruments::common_impl::models::correlation::copula`].
//!
//! Provides a trait-based copula abstraction enabling pluggable correlation
//! models while maintaining backward compatibility with one-factor Gaussian.
//!
//! # Supported Models
//!
//! - **Gaussian**: Standard one-factor Gaussian copula (market default)
//! - **Student-t**: Fat-tailed copula capturing tail dependence
//! - **Random Factor Loading (RFL)**: Stochastic correlation model
//! - **Multi-Factor**: Sector-based correlation structure
//!
//! # References
//!
//! - Li, D. X. (2000). "On Default Correlation: A Copula Function Approach."
//! - Andersen, L., & Sidenius, J. (2005). "Extensions to the Gaussian Copula"
//! - Hull, J., & White, A. (2004). "Valuation of a CDO without Monte Carlo"

// Re-export everything from the shared correlation module
pub use crate::instruments::common_impl::models::correlation::copula::*;
