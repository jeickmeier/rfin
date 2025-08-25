//! Numerical helpers: root finding, summation, and simple statistics.
//!
//! The implementations are `no_std` friendly and avoid allocation. When the
//! `deterministic` feature is enabled, functions prefer numerically stable,
//! order-preserving algorithms.

pub mod root_finding;
pub mod stats;
pub mod summation;

// Re-exports for ergonomic access
pub use root_finding::{brent, newton_bracketed};
pub use stats::{correlation, covariance, mean, mean_var, variance};
pub use summation::{kahan_sum, pairwise_sum, stable_sum};
