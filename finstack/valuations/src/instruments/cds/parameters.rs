//! Shared credit reference parameters used across CDS-related instruments.
//!
//! `CreditParams` captures the reference entity and recovery assumption used
//! for default settlement. Keep this lightweight and serde-stable; prefer
//! constructing with `new` or one of the standard presets.

/// ISDA-style recovery rate presets (named constants; do not hard-code literals).
pub const RECOVERY_SENIOR_UNSECURED: f64 = 0.40;
pub const RECOVERY_SUBORDINATED: f64 = 0.20;
/// Convenience non-ISDA preset for high-yield style assumptions (common heuristic).
pub const RECOVERY_HIGH_YIELD_DEFAULT: f64 = 0.30;

// Note: The CDS-specific CreditParams duplicated common::parameters::CreditParams and has been
// removed to avoid divergence. Use `crate::instruments::common::parameters::CreditParams` instead.
