//! Numerical helpers: root finding, summation, and simple statistics.
//!
//! The implementations avoid heap allocation. When the
//! `deterministic` feature is enabled, functions prefer numerically stable,
//! order-preserving algorithms.

pub mod root_finding;
pub mod stats;
pub mod summation;

// Re-exports for ergonomic access
pub use root_finding::{
    brent, brent_with_bracketing, find_bracketing_interval, newton_bracketed, newton_raphson,
};
pub use stats::{correlation, covariance, mean, mean_var, variance};
pub use summation::{kahan_sum, pairwise_sum, stable_sum};
