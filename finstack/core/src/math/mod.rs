//! Numerical helpers: root finding, summation, statistics, distributions, and mathematical functions.
//!
//! The implementations avoid heap allocation. When the
//! `deterministic` feature is enabled, functions prefer numerically stable,
//! order-preserving algorithms.

pub mod distributions;
pub mod integration;
pub mod random;
pub mod root_finding;
pub mod special_functions;
pub mod stats;
pub mod summation;

// Re-exports for ergonomic access
pub use distributions::{
    binomial_probability, log_binomial_coefficient, log_factorial, sample_beta,
};
pub use integration::GaussHermiteQuadrature;
pub use random::{RandomNumberGenerator, SimpleRng};
pub use root_finding::{
    brent, brent_with_bracketing, find_bracketing_interval, newton_bracketed, newton_raphson,
};
pub use special_functions::{
    erf, norm_cdf, norm_pdf, standard_normal_cdf, standard_normal_inv_cdf,
};
pub use stats::{correlation, covariance, mean, mean_var, variance};
pub use summation::{kahan_sum, pairwise_sum, stable_sum};
