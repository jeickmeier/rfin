//! Shared correlation infrastructure for credit modeling.
//!
//! This crate provides reusable correlation models used across credit instruments:
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
//! - [`factor_model::validate_correlation_matrix`]: Validate correlation matrices
//! - [`factor_model::cholesky_decompose`]: Cholesky decomposition for correlated factor generation
//! - [`joint_probability::correlation_bounds`]: Fréchet-Hoeffding bounds for correlated Bernoulli
//!
//! # Conventions
//!
//! - Probabilities, correlations, and recovery rates are quoted in decimals.
//! - Flattened correlation matrices use row-major ordering.
//! - Latent-factor inputs are standard-normal or Student-t realizations, depending
//!   on the concrete model.
//!
//! # References
//!
//! - Gaussian copula background:
//!   `docs/REFERENCES.md#li-2000-gaussian-copula`
//! - Student-t copula background:
//!   `docs/REFERENCES.md#demarta-mcneil-2005-t-copula`
//! - Factor-model and portfolio-risk context:
//!   `docs/REFERENCES.md#meucci-risk-and-asset-allocation`

pub mod copula;
pub mod error;
pub mod factor_model;
pub mod joint_probability;
pub mod recovery;

// Re-export commonly used types
pub use copula::{
    Copula, CopulaSpec, GaussianCopula, MultiFactorCopula, RandomFactorLoadingCopula,
    StudentTCopula,
};
pub use error::CorrelationMatrixError;
pub use factor_model::{
    cholesky_decompose, validate_correlation_matrix, FactorModel, FactorSpec, MultiFactorModel,
    SingleFactorModel, TwoFactorModel,
};
pub use joint_probability::{correlation_bounds, joint_probabilities, CorrelatedBernoulli};
pub use recovery::{ConstantRecovery, CorrelatedRecovery, RecoveryModel, RecoverySpec};
