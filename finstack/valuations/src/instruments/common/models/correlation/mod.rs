//! Shared correlation infrastructure for credit modeling.
//!
//! This module provides reusable correlation models used across credit instruments:
//! - CDS tranche pricing
//! - Structured credit (ABS/CLO/CMBS/RMBS)
//! - Portfolio credit risk
//!
//! # Components
//!
//! - [`copula`]: Copula models for default correlation (Gaussian, Student-t, RFL, Multi-factor)
//! - [`recovery`]: Recovery rate models (constant, market-correlated)
//! - [`factor_model`]: Factor models for correlated behavior
//! - [`joint_probability`]: Joint probability utilities for correlated events
//!
//! # Utilities
//!
//! - [`validate_correlation_matrix`]: Validate correlation matrices (delegates to core, rich error classification)
//! - [`cholesky_decompose`]: Cholesky decomposition for correlated factor generation
//! - [`correlation_bounds`]: Fréchet-Hoeffding bounds for correlated Bernoulli

pub mod copula;
pub mod factor_model;
pub mod joint_probability;
pub mod recovery;

// Re-export commonly used types
pub use copula::{
    Copula, CopulaSpec, GaussianCopula, MultiFactorCopula, RandomFactorLoadingCopula,
    StudentTCopula,
};
pub use factor_model::{
    cholesky_decompose, validate_correlation_matrix, CorrelationMatrixError, FactorModel,
    FactorSpec, MultiFactorModel, SingleFactorModel, TwoFactorModel,
};
pub use joint_probability::{correlation_bounds, joint_probabilities, CorrelatedBernoulli};
pub use recovery::{ConstantRecovery, CorrelatedRecovery, RecoveryModel, RecoverySpec};
