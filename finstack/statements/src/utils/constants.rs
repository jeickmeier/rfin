//! Shared numeric constants for the statements crate.
//!
//! Re-exports the workspace-wide `ZERO_TOLERANCE` from `finstack_core::math` as `EPSILON`
//! to maintain API compatibility.

/// Epsilon value for floating-point comparisons and near-zero detection (1e-10).
///
/// This is a re-export of `finstack_core::math::ZERO_TOLERANCE` under the name
/// used throughout the statements crate.
pub use finstack_core::math::ZERO_TOLERANCE as EPSILON;
