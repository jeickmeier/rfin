//! Variance-reduction utilities for Monte Carlo pricing.
//!
//! [`control_variate`] is always available. Antithetic pairing is implemented
//! inline in [`crate::engine::McEngine`] and configured via
//! [`crate::engine::McEngineConfig::antithetic`]. Under the `mc` feature this
//! module also exposes moment matching and importance sampling for richer
//! simulation workflows.
//!
//! Each leaf module documents the estimator assumptions, the quantity being
//! reweighted or paired, and the units of the returned diagnostics.

pub mod control_variate;

#[cfg(feature = "mc")]
pub mod moment_matching;

#[cfg(feature = "mc")]
pub mod importance_sampling;

pub use control_variate::{
    apply_control_variate, black_scholes_call, black_scholes_put, covariance,
};
