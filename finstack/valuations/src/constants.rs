//! Common constants used throughout the valuations crate.
//!
//! This module centralizes numerical constants to improve maintainability
//! and clarity across the codebase.

/// One basis point (0.01%) as a decimal.
///
/// Use this constant instead of hardcoded `0.0001` or `1e-4` for clarity
/// when calculating sensitivity to 1bp changes in rates, spreads, etc.
///
/// # Examples
/// ```rust
/// use finstack_valuations::constants::ONE_BASIS_POINT;
///
/// let rate_change = 100.0 * ONE_BASIS_POINT; // 1% or 100bp
/// let notional = 1_000_000.0;
/// let duration = 5.0;
/// let dv01 = notional * duration * ONE_BASIS_POINT;
/// assert_eq!(dv01, 500.0);
/// ```
pub const ONE_BASIS_POINT: f64 = 0.0001;

/// Basis points per unit (inverse of ONE_BASIS_POINT).
///
/// Use this to convert decimals to basis points.
/// # Examples
/// ```rust
/// use finstack_valuations::constants::BASIS_POINTS_PER_UNIT;
///
/// let spread_decimal = 0.0025;
/// let spread_bp = spread_decimal * BASIS_POINTS_PER_UNIT; // 25bp
/// ```
pub const BASIS_POINTS_PER_UNIT: f64 = 10_000.0;
