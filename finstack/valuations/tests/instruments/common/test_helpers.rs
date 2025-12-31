//! Shared test helpers and fixtures for common module tests.
//!
//! Provides:
//! - Standard market data fixtures
//! - Test curve builders
//! - Comparison utilities with tolerance
//! - Common test scenarios

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use time::Month;

/// Standard tolerance for numerical comparisons (0.01%)
pub const TOLERANCE: f64 = 1e-4;

/// Tight tolerance for exact calculations (0.0001%)
pub const TIGHT_TOLERANCE: f64 = 1e-6;

/// Relative tolerance for percentage checks (1%)
pub const RELATIVE_TOLERANCE: f64 = 0.01;

/// Tolerance tiers for different test categories.
///
/// Use these standardized tolerances instead of ad-hoc hardcoded values
/// to ensure consistency across the test suite.
///
/// ## Tolerance Hierarchy (strictest to loosest)
///
/// | Level | Constant | Value | Use Case |
/// |-------|----------|-------|----------|
/// | 1 | `ANALYTICAL` | 1e-6 (0.0001%) | Closed-form solutions (put-call parity, zero-coupon YTM) |
/// | 2 | `NUMERICAL` | 1e-4 (0.01%) | Iterative methods (Newton-Raphson, tree pricing) |
/// | 3 | `CURVE_PRICING` | 5e-3 (0.5%) | Curve-based valuations with convention differences |
/// | 4 | `RELATIVE` | 1e-2 (1%) | Proportional comparisons, textbook benchmarks |
/// | 5 | `BUMP_VS_ANALYTICAL` | 1.5e-2 (1.5%) | Bump-and-reprice vs analytical approximations |
/// | 6 | `STATISTICAL` | 2e-2 (2%) | Monte Carlo and statistical tests |
pub mod tolerances {
    /// Analytical calculations (e.g., put-call parity, zero-coupon YTM).
    /// These have closed-form solutions and should be very precise.
    pub const ANALYTICAL: f64 = 1e-6; // 0.0001%

    /// Numerical methods (e.g., tree pricing, Newton-Raphson solvers).
    /// These involve iterative convergence and may have small residual errors.
    pub const NUMERICAL: f64 = 1e-4; // 0.01%

    /// Curve-based pricing with potential convention mismatches.
    /// Accounts for compounding convention differences (e.g., semi-annual vs continuous).
    pub const CURVE_PRICING: f64 = 5e-3; // 0.5%

    /// Relative tolerance for scaling comparisons.
    /// Used when comparing proportional changes across different scales.
    pub const RELATIVE: f64 = 1e-2; // 1%

    /// Bump-and-reprice vs analytical approximation comparisons.
    ///
    /// Used when comparing numerical bump-based sensitivities (e.g., DV01 computed
    /// via curve parallel shift) against analytical approximations (e.g., DV01 ≈
    /// -Price × ModDur × 0.0001). The difference arises from:
    /// - Compounding convention mismatch (continuous vs periodic): ~0.6%
    /// - Curve-based vs yield-based rate definitions
    /// - Convexity effects (negligible for 1bp bumps)
    ///
    /// For par bonds on flat curves, actual differences are typically 0.5-1.5%.
    pub const BUMP_VS_ANALYTICAL: f64 = 1.5e-2; // 1.5%

    /// Statistical/Monte Carlo methods.
    /// These have inherent sampling variance.
    pub const STATISTICAL: f64 = 2e-2; // 2%
}

/// Scale tolerance with value magnitude, with a minimum absolute floor.
///
/// Useful for property tests where tolerance should scale with the value being tested
/// but should never go below a minimum threshold.
///
/// # Arguments
/// * `base_tol` - Base relative tolerance (e.g., 1e-4 for 0.01%)
/// * `value` - The value to scale against
/// * `min_abs` - Minimum absolute tolerance floor
///
/// # Returns
/// The larger of (value * base_tol) and min_abs
pub fn scaled_tolerance(base_tol: f64, value: f64, min_abs: f64) -> f64 {
    (value.abs() * base_tol).max(min_abs)
}

/// Assert two floats are approximately equal with given tolerance
pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64, msg: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff < tolerance,
        "{}: expected {}, got {} (diff: {})",
        msg,
        expected,
        actual,
        diff
    );
}

/// Assert two floats are approximately equal with relative tolerance
pub fn assert_relative_eq(actual: f64, expected: f64, rel_tolerance: f64, msg: &str) {
    if expected.abs() < 1e-10 {
        // For near-zero values, use absolute tolerance
        assert_approx_eq(actual, expected, 1e-10, msg);
    } else {
        let rel_diff = ((actual - expected) / expected).abs();
        assert!(
            rel_diff < rel_tolerance,
            "{}: expected {}, got {} (relative diff: {:.2}%)",
            msg,
            expected,
            actual,
            rel_diff * 100.0
        );
    }
}

/// Assert Money values are approximately equal
pub fn assert_money_eq(actual: Money, expected: Money, tolerance: f64, msg: &str) {
    assert_eq!(
        actual.currency(),
        expected.currency(),
        "{}: currency mismatch",
        msg
    );
    assert_approx_eq(actual.amount(), expected.amount(), tolerance, msg);
}

/// Create a standard test date (2025-01-01)
pub fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

/// Create a flat discount curve for testing
pub fn flat_curve(rate: f64, curve_id: &str) -> DiscountCurve {
    let base = test_date();
    let t_max = 30.0; // 30 years
    let df_max = (-rate * t_max).exp();

    DiscountCurve::builder(curve_id)
        .base_date(base)
        .knots([(0.0, 1.0), (t_max, df_max)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap()
}

/// Create a flat discount curve with custom rate and base date
pub fn flat_discount_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    let mut builder = DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
            (30.0, (-rate * 30.0).exp()),
        ]);

    // For negative rates, DFs are increasing (> 1), so we need:
    // - Linear interpolation (MonotoneConvex requires decreasing DFs)
    // - allow_non_monotonic flag
    if rate < 0.0 {
        builder = builder
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic();
    }

    builder.build().unwrap()
}

/// Create a flat hazard curve with recovery rate
#[allow(dead_code)]
pub fn flat_hazard_curve(
    hazard_rate: f64,
    recovery: f64,
    base_date: Date,
    curve_id: &str,
) -> HazardCurve {
    HazardCurve::builder(curve_id)
        .base_date(base_date)
        .recovery_rate(recovery)
        .day_count(DayCount::Act365F)
        .knots([(1.0, hazard_rate), (10.0, hazard_rate)])
        .build()
        .unwrap()
}

/// Standard test date helper that takes y/m/d
#[allow(dead_code)]
pub fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

/// Create a standard upward-sloping curve
pub fn upward_curve(curve_id: &str) -> DiscountCurve {
    let base = test_date();

    DiscountCurve::builder(curve_id)
        .base_date(base)
        .knots([
            (0.0, 1.0),
            (1.0, 0.97),
            (2.0, 0.94),
            (5.0, 0.85),
            (10.0, 0.70),
            (30.0, 0.40),
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap()
}

/// Create a standard market context with USD and EUR curves
pub fn standard_market() -> MarketContext {
    MarketContext::new()
        .insert_discount(flat_curve(0.05, "USD-OIS"))
        .insert_discount(flat_curve(0.052, "USD-SOFR-3M"))
        .insert_discount(flat_curve(0.03, "EUR-OIS"))
        .insert_discount(flat_curve(0.032, "EUR-EURIBOR-6M"))
}

/// Create test money in USD
pub fn usd(amount: f64) -> Money {
    Money::new(amount, Currency::USD)
}

/// Create test money in EUR
pub fn eur(amount: f64) -> Money {
    Money::new(amount, Currency::EUR)
}

/// Create test money in GBP
#[allow(dead_code)]
pub fn gbp(amount: f64) -> Money {
    Money::new(amount, Currency::GBP)
}

/// Standard day count for testing
pub fn standard_dc() -> DayCount {
    DayCount::Act365F
}

/// Calculate year fraction for testing
pub fn year_fraction(start: Date, end: Date) -> f64 {
    use finstack_core::dates::DayCountCtx;
    standard_dc()
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap_or(0.0)
}

/// Black-Scholes analytical solution for European call (for validation)
pub fn black_scholes_call(
    spot: f64,
    strike: f64,
    rate: f64,
    vol: f64,
    time: f64,
    div_yield: f64,
) -> f64 {
    use finstack_core::math::norm_cdf;

    if time <= 0.0 {
        return (spot - strike).max(0.0);
    }

    let sqrt_t = time.sqrt();
    let d1 = ((spot / strike).ln() + (rate - div_yield + 0.5 * vol * vol) * time) / (vol * sqrt_t);
    let d2 = d1 - vol * sqrt_t;

    let discount = (-rate * time).exp();
    let forward_discount = (-(rate - div_yield) * time).exp();

    spot * forward_discount * norm_cdf(d1) - strike * discount * norm_cdf(d2)
}

/// Black-Scholes analytical solution for European put (for validation)
pub fn black_scholes_put(
    spot: f64,
    strike: f64,
    rate: f64,
    vol: f64,
    time: f64,
    div_yield: f64,
) -> f64 {
    let call = black_scholes_call(spot, strike, rate, vol, time, div_yield);
    let pv_strike = strike * (-rate * time).exp();
    let pv_spot = spot * (-div_yield * time).exp();

    // Put-call parity: P = C - S*e^(-qT) + K*e^(-rT)
    call - pv_spot + pv_strike
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_eq() {
        assert_approx_eq(1.0001, 1.0, 0.001, "Should be approximately equal");
    }

    #[test]
    fn test_scaled_tolerance() {
        // For large values, tolerance should scale
        assert!((scaled_tolerance(1e-4, 1000.0, 0.01) - 0.1).abs() < 1e-10);

        // For small values, minimum floor should apply
        assert!((scaled_tolerance(1e-4, 0.001, 0.01) - 0.01).abs() < 1e-10);

        // Zero value uses minimum
        assert!((scaled_tolerance(1e-4, 0.0, 0.01) - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_tolerance_tiers_ordering() {
        // Verify tolerance tiers are in expected order (strictest to loosest)
        // Note: These are compile-time constants, so the ordering is verified at compile time.
        // Runtime assertions would be optimized out, so we document the expected ordering here.
        const _: () = {
            assert!(tolerances::ANALYTICAL < tolerances::NUMERICAL);
            assert!(tolerances::NUMERICAL < tolerances::CURVE_PRICING);
            assert!(tolerances::CURVE_PRICING < tolerances::RELATIVE);
            assert!(tolerances::RELATIVE < tolerances::BUMP_VS_ANALYTICAL);
            assert!(tolerances::BUMP_VS_ANALYTICAL < tolerances::STATISTICAL);
        };
    }

    #[test]
    #[should_panic]
    fn test_approx_eq_fails() {
        assert_approx_eq(1.1, 1.0, 0.01, "Should fail");
    }

    #[test]
    fn test_relative_eq() {
        assert_relative_eq(100.0, 99.5, RELATIVE_TOLERANCE, "Within relative tolerance");
    }

    #[test]
    fn test_money_helpers() {
        let base = usd(100.0);
        let bumped = usd(100.0 + TOLERANCE * 0.5);
        assert_money_eq(base, bumped, TOLERANCE, "USD helper within tolerance");

        let eur_value = eur(50.0);
        assert_eq!(eur_value.currency(), Currency::EUR);
    }

    #[test]
    fn test_upward_curve_builder() {
        let curve = upward_curve("UPWARD");
        assert_eq!(curve.id().as_str(), "UPWARD");
        // Later maturity should discount less than near term
        let df_short = curve.df(1.0);
        let df_long = curve.df(10.0);
        assert!(df_long < df_short);
    }

    #[test]
    fn test_year_fraction_helper() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let yf = year_fraction(start, end);
        assert!((yf - 1.0).abs() < 1e-4);
        assert_eq!(standard_dc(), DayCount::Act365F);
    }

    #[test]
    fn test_black_scholes_put_call_parity() {
        let spot = 100.0;
        let strike = 100.0;
        let rate = 0.05;
        let vol = 0.20;
        let time = 1.0;
        let div = 0.0;

        let call = black_scholes_call(spot, strike, rate, vol, time, div);
        let put = black_scholes_put(spot, strike, rate, vol, time, div);

        // Put-call parity: C - P = S - K*e^(-rT)
        let lhs = call - put;
        let rhs = spot - strike * (-rate * time).exp();

        assert_approx_eq(lhs, rhs, TIGHT_TOLERANCE, "Put-call parity");
    }

    #[test]
    fn test_flat_curve_creation() {
        let curve = flat_curve(0.05, "TEST");
        assert_eq!(curve.id().as_str(), "TEST");
        assert_eq!(curve.base_date(), test_date());
    }

    #[test]
    fn test_standard_market_has_curves() {
        let market = standard_market();
        assert!(market.get_discount("USD-OIS").is_ok());
        assert!(market.get_discount("EUR-OIS").is_ok());
    }
}
