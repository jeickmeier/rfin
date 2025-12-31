//! Shared test fixtures for the models test suite.
//!
//! Provides reusable builders for common test data structures like
//! discount curves, market contexts, and standard dates.

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::types::Currency;
use time::Month;

// =============================================================================
// Standard Test Dates
// =============================================================================

/// Standard base date for most calibration tests (2025-01-02).
///
/// This is a business day, avoiding holiday adjustment complications.
pub fn base_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 2).expect("valid test date: 2025-01-02")
}

/// IMM-style base date for CDS tests (March 20th).
///
/// ISDA CDS conventions use IMM dates (Mar 20, Jun 20, Sep 20, Dec 20).
#[allow(dead_code)]
pub fn imm_base_date() -> Date {
    Date::from_calendar_date(2025, Month::March, 20).expect("valid IMM date: 2025-03-20")
}

// =============================================================================
// Discount Curve Fixtures
// =============================================================================

/// Creates a standard USD discount curve for testing.
///
/// # Arguments
/// * `base_date` - The curve's base/valuation date
/// * `curve_id` - Identifier for the curve (e.g., "USD-OIS")
///
/// # Returns
/// A discount curve with typical short/medium/long tenors.
#[allow(dead_code)]
pub fn usd_discount_curve(base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (1.0, 0.98),
            (3.0, 0.94),
            (5.0, 0.90),
            (10.0, 0.80),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("valid test discount curve")
}

/// Creates a minimal USD discount curve with fewer knots.
///
/// Useful for tests that don't need a full term structure.
pub fn usd_discount_curve_minimal(base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("valid minimal test discount curve")
}

/// Creates a discount curve with monotone convex interpolation.
///
/// Suitable for forward curve calibration tests where smooth forwards matter.
#[allow(dead_code)]
pub fn usd_discount_curve_monotone_convex(base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.9888),
            (0.5, 0.9775),
            (1.0, 0.9550),
            (2.0, 0.9100),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .expect("valid monotone convex test discount curve")
}

// =============================================================================
// Market Context Fixtures
// =============================================================================

/// Creates a market context with a standard USD-OIS discount curve.
#[allow(dead_code)]
pub fn market_context_with_usd_discount(base_date: Date) -> MarketContext {
    let curve = usd_discount_curve(base_date, "USD-OIS");
    MarketContext::new().insert_discount(curve)
}

/// Creates a market context with a minimal USD-OIS discount curve.
#[allow(dead_code)]
pub fn market_context_with_minimal_discount(base_date: Date) -> MarketContext {
    let curve = usd_discount_curve_minimal(base_date, "USD-OIS");
    MarketContext::new().insert_discount(curve)
}

// =============================================================================
// Build Context Helpers
// =============================================================================

/// Standard notional for test instruments ($1M).
#[allow(dead_code)]
pub const STANDARD_NOTIONAL: f64 = 1_000_000.0;

/// Standard currency for USD tests.
#[allow(dead_code)]
pub const USD: Currency = Currency::USD;

// =============================================================================
// Test Assertion Helpers
// =============================================================================

/// Strict absolute tolerance for pure floating-point identity checks.
pub const F64_ABS_TOL_STRICT: f64 = 1e-12;

/// Looser absolute tolerance for computations with multiple steps.
#[allow(dead_code)]
pub const F64_ABS_TOL_LOOSE: f64 = 1e-10;

/// Assert two f64 values are close within strict tolerance.
#[allow(dead_code)]
pub fn assert_close_strict(actual: f64, expected: f64, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= F64_ABS_TOL_STRICT,
        "{label}: |actual-expected|={diff:.3e} > tol={:.3e} (actual={actual:.12}, expected={expected:.12})",
        F64_ABS_TOL_STRICT
    );
}

/// Assert two f64 values are close within loose tolerance.
#[allow(dead_code)]
pub fn assert_close_loose(actual: f64, expected: f64, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= F64_ABS_TOL_LOOSE,
        "{label}: |actual-expected|={diff:.3e} > tol={:.3e} (actual={actual:.12}, expected={expected:.12})",
        F64_ABS_TOL_LOOSE
    );
}
