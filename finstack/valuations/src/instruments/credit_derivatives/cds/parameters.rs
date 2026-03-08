//! Shared credit reference parameters used across CDS-related instruments.
//!
//! `CreditParams` captures the reference entity and recovery assumption used
//! for default settlement. Keep this lightweight and serde-stable; prefer
//! constructing with `new` or one of the standard presets.
//!
//! Recovery rate constants are sourced from [`crate::constants::isda`] to avoid
//! duplication. CDS-conventional aliases are re-exported here for ergonomics.

/// ISDA standard recovery rate for senior unsecured (40%).
///
/// Re-exported from [`crate::constants::isda::STANDARD_RECOVERY_SENIOR`].
pub use crate::constants::isda::STANDARD_RECOVERY_SENIOR as RECOVERY_SENIOR_UNSECURED;

/// ISDA standard recovery rate for subordinated (20%).
///
/// Re-exported from [`crate::constants::isda::STANDARD_RECOVERY_SUB`].
pub use crate::constants::isda::STANDARD_RECOVERY_SUB as RECOVERY_SUBORDINATED;

/// Convenience non-ISDA preset for high-yield style assumptions (common heuristic, 30%).
pub const RECOVERY_HIGH_YIELD_DEFAULT: f64 = 0.30;

// Note: The CDS-specific CreditParams duplicated common::parameters::CreditParams and has been
// removed to avoid divergence. Use `crate::instruments::common_impl::parameters::CreditParams` instead.
