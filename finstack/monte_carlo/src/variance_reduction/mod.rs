//! Variance-reduction utilities for Monte Carlo pricing.
//!
//! [`antithetic`] and [`control_variate`] are always available and cover the two
//! most common reductions for vanilla pricing. Under the `mc` feature this
//! module also exposes moment matching and importance sampling for richer
//! simulation workflows.
//!
//! Each leaf module documents the estimator assumptions, the quantity being
//! reweighted or paired, and the units of the returned diagnostics.

pub mod antithetic;
pub mod control_variate;

#[cfg(feature = "mc")]
pub mod moment_matching;

#[cfg(feature = "mc")]
pub mod importance_sampling;

pub use antithetic::{antithetic_price, AntitheticConfig};
pub use control_variate::{
    apply_control_variate, black_scholes_call, black_scholes_put, covariance,
};
