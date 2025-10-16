//! Common test utilities and fixtures for ILB tests

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::inflation_index::{
    InflationIndex, InflationInterpolation, InflationLag,
};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::inflation_linked_bond::{
    DeflationProtection, IndexationMethod, InflationLinkedBond,
};
use time::Month;

/// Shorthand for date creation
pub fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

/// Create a standard TIPS bond for testing
pub fn sample_tips() -> InflationLinkedBond {
    InflationLinkedBond::builder()
        .id(InstrumentId::new("TIPS-TEST"))
        .notional(Money::new(1_000_000.0, Currency::USD))
        .real_coupon(0.0125) // 1.25% real coupon
        .freq(Frequency::semi_annual())
        .dc(DayCount::ActAct)
        .issue(d(2020, 1, 15))
        .maturity(d(2030, 1, 15))
        .base_index(250.0)
        .base_date(d(2019, 10, 1))
        .indexation_method(IndexationMethod::TIPS)
        .lag(InflationLag::Months(3))
        .deflation_protection(DeflationProtection::MaturityOnly)
        .bdc(BusinessDayConvention::Following)
        .stub(StubKind::None)
        .disc_id(CurveId::new("USD-REAL"))
        .inflation_id(CurveId::new("US-CPI-U"))
        .attributes(Attributes::new())
        .build()
        .unwrap()
}

/// Create a UK Index-Linked Gilt for testing
pub fn sample_uk_linker() -> InflationLinkedBond {
    InflationLinkedBond::builder()
        .id(InstrumentId::new("UK-GILT-TEST"))
        .notional(Money::new(1_000_000.0, Currency::GBP))
        .real_coupon(0.00625) // 0.625% real coupon
        .freq(Frequency::semi_annual())
        .dc(DayCount::ActAct)
        .issue(d(2020, 3, 22))
        .maturity(d(2040, 3, 22))
        .base_index(280.0)
        .base_date(d(2019, 7, 1))
        .indexation_method(IndexationMethod::UK)
        .lag(InflationLag::Months(8))
        .deflation_protection(DeflationProtection::None)
        .bdc(BusinessDayConvention::Following)
        .stub(StubKind::None)
        .disc_id(CurveId::new("GBP-NOMINAL"))
        .inflation_id(CurveId::new("UK-RPI"))
        .attributes(Attributes::new())
        .build()
        .unwrap()
}

/// Create a market context with inflation index (linear interpolation for TIPS)
pub fn market_context_with_index() -> (MarketContext, InflationIndex) {
    let disc = DiscountCurve::builder("USD-REAL")
        .base_date(d(2025, 1, 2))
        .knots([
            (0.0, 1.0),
            (0.5, 0.99),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.90),
            (10.0, 0.82),
        ])
        .build()
        .unwrap();

    let observations = vec![
        (d(2024, 11, 1), 299.0),
        (d(2024, 12, 1), 300.0),
        (d(2025, 1, 1), 301.0),
        (d(2025, 2, 1), 302.0),
        (d(2025, 3, 1), 303.0),
    ];
    let index = InflationIndex::new("US-CPI-U", observations, Currency::USD)
        .unwrap()
        .with_interpolation(InflationInterpolation::Linear);

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation_index("US-CPI-U", index.clone());

    (ctx, index)
}

/// Create a market context with inflation curve (for forward projections)
pub fn market_context_with_curve() -> (MarketContext, InflationCurve) {
    let disc = DiscountCurve::builder("USD-REAL")
        .base_date(d(2025, 1, 2))
        .knots([
            (0.0, 1.0),
            (0.5, 0.99),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.90),
            (10.0, 0.82),
        ])
        .build()
        .unwrap();

    // Inflation curve with 2% p.a. growth
    let curve = InflationCurve::builder("US-CPI-U")
        .base_cpi(300.0)
        .knots([
            (0.0, 300.0),
            (0.5, 303.0),
            (1.0, 306.0),
            (2.0, 312.0),
            (5.0, 330.0),
            (10.0, 366.0),
        ])
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation(curve);

    (
        ctx,
        InflationCurve::builder("US-CPI-U")
            .base_cpi(300.0)
            .knots([
                (0.0, 300.0),
                (0.5, 303.0),
                (1.0, 306.0),
                (2.0, 312.0),
                (5.0, 330.0),
                (10.0, 366.0),
            ])
            .build()
            .unwrap(),
    )
}

/// Create a UK market context with step interpolation
pub fn uk_market_context() -> (MarketContext, InflationIndex) {
    let disc = DiscountCurve::builder("GBP-NOMINAL")
        .base_date(d(2025, 1, 2))
        .knots([
            (0.0, 1.0),
            (0.5, 0.985),
            (1.0, 0.97),
            (2.0, 0.94),
            (5.0, 0.87),
            (15.0, 0.65),
        ])
        .build()
        .unwrap();

    let observations = vec![
        (d(2024, 11, 1), 319.0),
        (d(2024, 12, 1), 320.0),
        (d(2025, 1, 1), 321.0),
        (d(2025, 2, 1), 322.0),
    ];
    let index = InflationIndex::new("UK-RPI", observations, Currency::GBP)
        .unwrap()
        .with_interpolation(InflationInterpolation::Step);

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_inflation_index("UK-RPI", index.clone());

    (ctx, index)
}

/// Tolerance for floating point comparisons
pub const EPSILON: f64 = 1e-9;

/// Relative tolerance for financial calculations
pub const REL_TOL: f64 = 1e-6;

/// Helper to check relative difference
pub fn relative_diff(a: f64, b: f64) -> f64 {
    if b.abs() < EPSILON {
        (a - b).abs()
    } else {
        ((a - b) / b).abs()
    }
}

/// Assert two values are approximately equal
#[track_caller]
pub fn assert_approx_eq(a: f64, b: f64, tol: f64, msg: &str) {
    let diff = relative_diff(a, b);
    assert!(
        diff < tol,
        "{}: expected {}, got {} (rel diff: {})",
        msg,
        b,
        a,
        diff
    );
}
