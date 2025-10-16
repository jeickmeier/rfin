//! Shared test fixtures and helpers for inflation swap tests.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::{
    InflationIndex, InflationInterpolation, InflationLag,
};
use finstack_core::market_data::term_structures::{DiscountCurve, InflationCurve};
use finstack_core::money::Money;

/// Build a flat discount curve with given rate
pub fn flat_discount(id: &str, base: Date, rate: f64) -> DiscountCurve {
    let knots = vec![
        (0.0, 1.0),
        (1.0, (-rate).exp() as f64),
        (5.0, (-rate * 5.0).exp() as f64),
        (10.0, (-rate * 10.0).exp() as f64),
        (30.0, (-rate * 30.0).exp() as f64),
    ];
    DiscountCurve::builder(id)
        .base_date(base)
        .day_count(DayCount::Act365F)
        .knots(knots)
        .build()
        .unwrap()
}

/// Build a flat inflation curve with constant CPI growth rate
pub fn flat_inflation_curve(id: &str, base_cpi: f64, annual_inflation_rate: f64) -> InflationCurve {
    let knots = vec![
        (0.0, base_cpi),
        (1.0, base_cpi * (1.0 + annual_inflation_rate)),
        (5.0, base_cpi * (1.0 + annual_inflation_rate).powf(5.0)),
        (10.0, base_cpi * (1.0 + annual_inflation_rate).powf(10.0)),
        (30.0, base_cpi * (1.0 + annual_inflation_rate).powf(30.0)),
    ];
    InflationCurve::builder(id)
        .base_cpi(base_cpi)
        .knots(knots)
        .build()
        .unwrap()
}

/// Build a realistic inflation curve with term structure
pub fn realistic_inflation_curve(id: &str, base_cpi: f64) -> InflationCurve {
    // Realistic forward inflation: front-end 2.5%, mid 2.0%, long 1.8%
    let knots = vec![
        (0.0, base_cpi),
        (0.25, base_cpi * 1.00625), // 2.5% for Q1
        (0.5, base_cpi * 1.0125),   // 2.5% annualized
        (1.0, base_cpi * 1.025),    // 2.5% at 1Y
        (2.0, base_cpi * 1.047),    // ~2.3% avg to 2Y
        (5.0, base_cpi * 1.105),    // ~2.0% avg to 5Y
        (10.0, base_cpi * 1.197),   // ~1.85% avg to 10Y
        (30.0, base_cpi * 1.703),   // ~1.8% avg to 30Y
    ];
    InflationCurve::builder(id)
        .base_cpi(base_cpi)
        .knots(knots)
        .build()
        .unwrap()
}

/// Build a simple inflation index with historical observations
pub fn simple_index(
    id: &str,
    base: Date,
    base_cpi: f64,
    ccy: Currency,
    lag: InflationLag,
) -> InflationIndex {
    let observations = vec![
        (base - time::Duration::days(180), base_cpi * 0.99),
        (base - time::Duration::days(150), base_cpi * 0.992),
        (base - time::Duration::days(120), base_cpi * 0.994),
        (base - time::Duration::days(90), base_cpi * 0.996),
        (base - time::Duration::days(60), base_cpi * 0.998),
        (base - time::Duration::days(30), base_cpi * 0.999),
        (base, base_cpi),
    ];
    InflationIndex::new(id, observations, ccy)
        .unwrap()
        .with_interpolation(InflationInterpolation::Linear)
        .with_lag(lag)
}

/// Build a comprehensive market context for testing
pub fn standard_market(as_of: Date, inflation_rate: f64, discount_rate: f64) -> MarketContext {
    let disc = flat_discount("USD-OIS", as_of, discount_rate);
    let infl_curve = flat_inflation_curve("US-CPI-U", 300.0, inflation_rate);
    let index = simple_index(
        "US-CPI-U",
        as_of,
        300.0,
        Currency::USD,
        InflationLag::Months(3),
    );

    MarketContext::new()
        .insert_discount(disc)
        .insert_inflation(infl_curve)
        .insert_inflation_index("US-CPI-U", index)
}

/// Build a market with realistic curves
pub fn realistic_market(as_of: Date) -> MarketContext {
    let disc = flat_discount("USD-OIS", as_of, 0.045); // 4.5% nominal rate
    let infl_curve = realistic_inflation_curve("US-CPI-U", 300.0);
    let index = simple_index(
        "US-CPI-U",
        as_of,
        300.0,
        Currency::USD,
        InflationLag::Months(3),
    );

    MarketContext::new()
        .insert_discount(disc)
        .insert_inflation(infl_curve)
        .insert_inflation_index("US-CPI-U", index)
}

/// Standard notional for tests
pub fn standard_notional() -> Money {
    Money::new(1_000_000.0, Currency::USD)
}

/// Large notional for sensitivity tests
pub fn large_notional() -> Money {
    Money::new(100_000_000.0, Currency::USD)
}

/// Tolerance for PV checks (0.01 basis point of notional)
pub fn pv_tolerance(notional: Money) -> f64 {
    notional.amount() * 1e-6
}

/// Tolerance for rate checks (0.01 bp)
pub fn rate_tolerance() -> f64 {
    1e-6
}

/// Tolerance for greek checks (relative)
pub fn greek_tolerance() -> f64 {
    0.05 // 5% relative tolerance for approximations
}
