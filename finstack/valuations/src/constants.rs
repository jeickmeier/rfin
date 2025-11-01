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

/// Convert percentage to decimal (1% = 0.01).
///
/// Use this constant when converting percentage values to decimal form.
/// # Examples
/// ```rust
/// use finstack_valuations::constants::PERCENT_TO_DECIMAL;
///
/// let rate_pct = 5.0; // 5%
/// let rate_decimal = rate_pct * PERCENT_TO_DECIMAL; // 0.05
/// ```
pub const PERCENT_TO_DECIMAL: f64 = 0.01;

/// Convert decimal to percentage (0.01 = 1%).
///
/// Use this constant when converting decimal values to percentage form.
/// # Examples
/// ```rust
/// use finstack_valuations::constants::DECIMAL_TO_PERCENT;
///
/// let rate_decimal = 0.05;
/// let rate_pct = rate_decimal * DECIMAL_TO_PERCENT; // 5.0%
/// ```
pub const DECIMAL_TO_PERCENT: f64 = 100.0;

/// Decimal-friendly helpers for deterministic arithmetic.
/// These supplement f64 constants without breaking existing callers.
/// Prefer these in money and aggregation code paths.
#[inline]
pub fn one_basis_point_dec() -> rust_decimal::Decimal {
    // 0.0001
    rust_decimal::Decimal::new(1, 4)
}

#[inline]
pub fn basis_points_per_unit_dec() -> rust_decimal::Decimal {
    // 10,000
    rust_decimal::Decimal::new(10_000, 0)
}

#[inline]
pub fn percent_to_decimal_dec() -> rust_decimal::Decimal {
    // 0.01
    rust_decimal::Decimal::new(1, 2)
}

#[inline]
pub fn decimal_to_percent_dec() -> rust_decimal::Decimal {
    // 100
    rust_decimal::Decimal::new(100, 0)
}
