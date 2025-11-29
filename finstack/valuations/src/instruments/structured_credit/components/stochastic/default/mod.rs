//! Stochastic default models for structured credit.
//!
//! This module provides factor-driven default models that capture:
//! - Default correlation through copula models
//! - Intensity process dynamics (Cox process)
//! - Factor-correlated CDR
//!
//! # Models
//!
//! - **CopulaBasedDefault**: Default correlation via copula (Gaussian, Student-t)
//! - **IntensityProcessDefault**: Cox process with mean-reverting intensity
//! - **FactorCorrelatedDefault**: Simple factor-shocked CDR
//!
//! # References
//!
//! - Li, D. X. (2000). "On Default Correlation: A Copula Function Approach."
//! - Duffie, D., & Singleton, K. J. (1999). "Modeling Term Structures of Defaultable Bonds."
//! - Schönbucher, P. J. (2003). "Credit Derivatives Pricing Models."

mod copula_based;
mod intensity_process;
mod spec;
mod traits;

pub use copula_based::CopulaBasedDefault;
pub use intensity_process::IntensityProcessDefault;
pub use spec::StochasticDefaultSpec;
pub use traits::{MacroCreditFactors, StochasticDefault};
