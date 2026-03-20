//! Joint probability utilities re-exported from `finstack_core`.
//!
//! This facade exposes the Bernoulli-coupling helpers most commonly used by the
//! correlation crate:
//! - [`joint_probabilities`] to construct a two-name joint distribution from
//!   marginals plus a feasible correlation
//! - [`correlation_bounds`] to compute the admissible Fréchet-Hoeffding range
//! - [`CorrelatedBernoulli`] for a reusable correlated Bernoulli wrapper
//!
//! Inputs are quoted in decimals. Marginal probabilities are in `[0, 1]`, and
//! the requested Bernoulli correlation is clamped to the feasible range implied
//! by those marginals.
//!
//! For the full implementation details and symbol-level docs, see
//! [`finstack_core::math::probability`].

pub use finstack_core::math::probability::{
    correlation_bounds, joint_probabilities, CorrelatedBernoulli,
};
