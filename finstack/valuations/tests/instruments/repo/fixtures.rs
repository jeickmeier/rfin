//! Common test fixtures and utilities for repo tests.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::instruments::rates::repo::CollateralSpec;
use time::Month;

/// Create a test date from year, month, day.
pub fn date(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

/// Standard test base date (2025-01-01).
pub fn base_date() -> Date {
    date(2025, 1, 1)
}

/// Create a standard USD OIS discount curve for testing.
pub fn create_usd_discount_curve() -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date())
        .knots([
            (0.0, 1.0),
            (0.25, 0.9875),
            (0.5, 0.975),
            (1.0, 0.95),
            (2.0, 0.90),
            (5.0, 0.78),
            (10.0, 0.60),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

/// Create a steep discount curve for sensitivity testing.
#[allow(dead_code)]
pub fn create_steep_discount_curve() -> DiscountCurve {
    DiscountCurve::builder("USD-OIS-STEEP")
        .base_date(base_date())
        .knots([(0.0, 1.0), (0.25, 0.98), (1.0, 0.92), (5.0, 0.70)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

/// Create a flat discount curve (useful for isolating interest calculations).
pub fn create_flat_discount_curve() -> DiscountCurve {
    // Flat curve for testing (zero rates) - requires allow_non_monotonic()
    DiscountCurve::builder("USD-FLAT")
        .base_date(base_date())
        .knots([(0.0, 1.0), (10.0, 1.0)])
        .set_interp(InterpStyle::Linear)
        .allow_non_monotonic()
        .build()
        .unwrap()
}

/// Create a standard market context with curves and prices.
pub fn create_standard_market_context() -> MarketContext {
    MarketContext::new()
        .insert_discount(create_usd_discount_curve())
        .insert_price(
            "TREASURY_BOND_PRICE",
            MarketScalar::Price(Money::new(1.02, Currency::USD)),
        )
        .insert_price(
            "CORPORATE_BOND_PRICE",
            MarketScalar::Price(Money::new(0.98, Currency::USD)),
        )
        .insert_price(
            "SPECIAL_BOND_PRICE",
            MarketScalar::Price(Money::new(1.05, Currency::USD)),
        )
        .insert_price(
            "HIGH_YIELD_PRICE",
            MarketScalar::Price(Money::new(0.85, Currency::USD)),
        )
        .insert_price(
            "EQUITY_PRICE",
            MarketScalar::Price(Money::new(150.0, Currency::USD)),
        )
}

/// Create general collateral backed by treasury bonds.
pub fn treasury_collateral() -> CollateralSpec {
    CollateralSpec::new("TREASURY_BOND", 1_000_000.0, "TREASURY_BOND_PRICE")
}

/// Create collateral backed by corporate bonds.
pub fn corporate_collateral() -> CollateralSpec {
    CollateralSpec::new("CORPORATE_BOND", 1_000_000.0, "CORPORATE_BOND_PRICE")
}

/// Create special collateral with rate adjustment.
pub fn special_collateral(rate_adjustment_bp: f64) -> CollateralSpec {
    CollateralSpec::special(
        "SPECIAL_BOND_ID",
        "SPECIAL_BOND",
        500_000.0,
        "SPECIAL_BOND_PRICE",
        Some(rate_adjustment_bp),
    )
}

/// Create undercollateralized position.
pub fn insufficient_collateral() -> CollateralSpec {
    CollateralSpec::new("HIGH_YIELD", 1_000_000.0, "HIGH_YIELD_PRICE")
}

/// Verify that two Money values are approximately equal.
pub fn assert_money_approx_eq(actual: Money, expected: Money, epsilon: f64) {
    assert_eq!(actual.currency(), expected.currency(), "Currency mismatch");
    let diff = (actual.amount() - expected.amount()).abs();
    assert!(
        diff < epsilon,
        "Expected ~{} {}, got {} {} (diff: {})",
        expected.amount(),
        expected.currency(),
        actual.amount(),
        actual.currency(),
        diff
    );
}

/// Verify that two f64 values are approximately equal.
pub fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff < epsilon,
        "Expected ~{}, got {} (diff: {})",
        expected,
        actual,
        diff
    );
}
