//! Credit risk modeling primitives.
//!
//! Currently provides the [`crate::credit::migration`] module for credit
//! migration modeling (JLT / CreditMetrics-style transition matrices and
//! CTMC simulation).
//!
//! Future phases will add:
//! - Time-inhomogeneous generators (economic-state-conditional migration).
//! - Full JLT model with stochastic migration intensities.
//! - Correlated multi-obligor simulation (CreditMetrics factor model).

/// Credit migration: transition matrices, generator extraction, projection,
/// and CTMC path simulation.
pub mod migration;
