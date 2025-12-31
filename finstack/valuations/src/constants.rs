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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn one_basis_point_dec() -> rust_decimal::Decimal {
    // 0.0001
    rust_decimal::Decimal::new(1, 4)
}

/// Basis points per unit (10,000) as Decimal
#[inline]
#[allow(dead_code)]
pub fn basis_points_per_unit_dec() -> rust_decimal::Decimal {
    // 10,000
    rust_decimal::Decimal::new(10_000, 0)
}

/// Conversion factor from percentage to decimal (0.01) as Decimal
#[inline]
#[allow(dead_code)]
pub fn percent_to_decimal_dec() -> rust_decimal::Decimal {
    // 0.01
    rust_decimal::Decimal::new(1, 2)
}

/// Conversion factor from decimal to percentage (100) as Decimal
#[inline]
#[allow(dead_code)]
pub fn decimal_to_percent_dec() -> rust_decimal::Decimal {
    // 100
    rust_decimal::Decimal::new(100, 0)
}

/// Tolerance for numerical calculations
pub const NUMERICAL_TOLERANCE: f64 = 1e-10;

/// Numerical constants for floating-point comparisons and integration.
///
/// These constants replace magic numbers scattered throughout the codebase,
/// providing consistent tolerances and step sizes with documented rationale.
pub mod numerical {
    /// Tolerance for checking if a value is effectively zero.
    ///
    /// Used for comparisons like `if lambda.abs() > ZERO_TOLERANCE` to avoid
    /// division by zero or special-case handling for near-zero values.
    ///
    /// Value: 1e-10 (chosen to be well above f64 machine epsilon ~2.2e-16
    /// but small enough to catch actual zeros vs meaningful small values).
    pub const ZERO_TOLERANCE: f64 = 1e-10;

    /// Step size factor for numerical differentiation and integration.
    ///
    /// When computing finite differences or integration steps, multiply the
    /// interval length by this factor: `h = (t_end - t_start) * INTEGRATION_STEP_FACTOR`.
    ///
    /// Value: 1e-4 (provides good balance between numerical stability and
    /// truncation error for typical financial time horizons of 0.1-30 years).
    pub const INTEGRATION_STEP_FACTOR: f64 = 1e-4;

    /// Tolerance for iterative solver convergence (bootstrap, calibration).
    ///
    /// Used as the convergence criterion for root-finding algorithms like
    /// Brent's method: stop when |f(x)| < SOLVER_TOLERANCE.
    ///
    /// Value: 1e-8 (tight enough for financial precision while avoiding
    /// excessive iterations for well-conditioned problems).
    pub const SOLVER_TOLERANCE: f64 = 1e-8;

    /// Tolerance for comparing floating-point rates and spreads.
    ///
    /// Used when checking if two rates are "equal" for purposes like
    /// detecting unchanged spreads or matching calibration targets.
    ///
    /// Value: 1e-12 (tighter than ZERO_TOLERANCE because rates are typically
    /// O(0.01) to O(0.1), so relative precision matters more).
    #[allow(dead_code)]
    pub const RATE_COMPARISON_TOLERANCE: f64 = 1e-12;

    /// Small epsilon to prevent division by zero.
    ///
    /// Add to denominators when there's risk of division by zero:
    /// `result = numerator / (denominator + DIVISION_EPSILON)`.
    ///
    /// Value: 1e-15 (close to but above f64 machine epsilon to ensure
    /// the addition is numerically meaningful).
    #[allow(dead_code)]
    pub const DIVISION_EPSILON: f64 = 1e-15;

    /// Default relative tolerance for numerical comparisons.
    ///
    /// Used for relative error checks: `|a - b| / max(|a|, |b|) < RELATIVE_TOLERANCE`.
    ///
    /// Value: 1e-9 (provides ~9 significant digits of precision).
    #[allow(dead_code)]
    pub const RELATIVE_TOLERANCE: f64 = 1e-9;
}

/// ISDA 2014 standard constants used by the engine
pub mod isda {
    /// Standard recovery rate for senior unsecured (40%)
    #[allow(dead_code)]
    pub const STANDARD_RECOVERY_SENIOR: f64 = 0.40;

    /// Standard recovery rate for subordinated (20%)
    #[allow(dead_code)]
    pub const STANDARD_RECOVERY_SUB: f64 = 0.20;

    /// Standard integration points per year for protection leg
    pub const STANDARD_INTEGRATION_POINTS: usize = 40;

    /// Standard coupon payment day
    #[allow(dead_code)]
    pub const STANDARD_COUPON_DAY: u8 = 20;
}

/// Business days per year constants by market region
pub mod time {
    /// Business days per year for North America (US markets)
    pub const BUSINESS_DAYS_PER_YEAR_US: f64 = 252.0;

    /// Business days per year for Europe (UK markets)
    pub const BUSINESS_DAYS_PER_YEAR_UK: f64 = 250.0;

    /// Business days per year for Asia (Japan markets)
    pub const BUSINESS_DAYS_PER_YEAR_JP: f64 = 255.0;
}
