//! Greeks calculation via Monte Carlo.
//!
//! Implements pathwise differentiation, likelihood ratio method,
//! and finite differences with common random numbers.

#[cfg(feature = "mc")]
pub mod pathwise;

#[cfg(feature = "mc")]
pub mod lrm;

#[cfg(feature = "mc")]
pub mod finite_diff;

