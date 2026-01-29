//! Unit tests for risk metrics (Duration, Z-spread, CS01, YTM).
//!
//! Tests cover:
//! - Duration calculations from dated cashflows
//! - Z-spread solver convergence
//! - CS01 price sensitivity

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_z_spread,
};
use time::Month;

fn base_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

fn flat_discount_curve(rate: f64) -> DiscountCurve {
    let base = base_date();
    DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (2.0, (-rate * 2.0).exp()),
            (5.0, (-rate * 5.0).exp()),
        ])
        .build()
        .unwrap()
}

fn sample_cashflows() -> Vec<(Date, Money)> {
    vec![
        (
            Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            Money::new(60_000.0, Currency::USD),
        ),
        (
            Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            Money::new(40_000.0, Currency::USD),
        ),
    ]
}

#[test]
fn test_tranche_duration_matches_weighted_pv_time() {
    let as_of = base_date();
    let curve = flat_discount_curve(0.05);
    let flows = sample_cashflows();

    let day_count = DayCount::Act365F;
    let mut pv = 0.0;
    let mut weighted_pv = 0.0;

    for (date, amount) in &flows {
        let t = day_count
            .year_fraction(as_of, *date, DayCountCtx::default())
            .unwrap();
        let df = curve.df_between_dates(as_of, *date).unwrap();
        let flow_pv = amount.amount() * df;
        pv += flow_pv;
        weighted_pv += flow_pv * t;
    }

    let expected_duration = weighted_pv / pv;
    let duration =
        calculate_tranche_duration(&flows, &curve, as_of, Money::new(pv, Currency::USD)).unwrap();

    assert!(
        (duration - expected_duration).abs() < 1e-4,
        "Duration should match PV-weighted average time"
    );
}

#[test]
fn test_z_spread_zero_for_curve_pv() {
    let as_of = base_date();
    let curve = flat_discount_curve(0.05);
    let flows = sample_cashflows();

    let mut pv = 0.0;
    for (date, amount) in &flows {
        let df = curve.df_between_dates(as_of, *date).unwrap();
        pv += amount.amount() * df;
    }

    let z_spread_bps =
        calculate_tranche_z_spread(&flows, &curve, Money::new(pv, Currency::USD), as_of).unwrap();

    assert!(
        z_spread_bps.abs() < 0.1,
        "Z-spread should be near 0 for curve-implied PV"
    );
}

#[test]
fn test_cs01_positive_for_spread_bump() {
    let as_of = base_date();
    let curve = flat_discount_curve(0.05);
    let flows = sample_cashflows();

    let cs01 = calculate_tranche_cs01(&flows, &curve, 0.0, as_of).unwrap();
    assert!(cs01 > 0.0, "CS01 should be positive for spread bumps");
}
