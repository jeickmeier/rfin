//! Additional integration tests for day count conventions.
//!
//! This module focuses on:
//! - Calendar integration (Bus/252 with real calendars)
//! - ISMA-specific behavior (ActActIsma with frequencies)
//! - Cross-convention comparisons
//!
//! Basic day count functionality is tested in unit tests at src/dates/daycount.rs

use finstack_core::dates::calendar::TARGET2;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Frequency};
use time::Month;

fn make_date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
}

// =============================================================================
// ISMA Convention Tests - Tests frequency-dependent behavior
// =============================================================================

#[test]
fn actact_isma_requires_frequency() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 7, 1);

    // Without frequency, should error
    let result = DayCount::ActActIsma.year_fraction(start, end, DayCountCtx::default());
    assert!(result.is_err());

    // With frequency, should work
    let ctx = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(6)),
        bus_basis: None,
    };
    let yf = DayCount::ActActIsma.year_fraction(start, end, ctx).unwrap();
    assert!(yf > 0.0);
}

#[test]
fn actact_isma_full_coupon_period() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 7, 1);

    let ctx = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(6)),
        bus_basis: None,
    };

    let yf = DayCount::ActActIsma.year_fraction(start, end, ctx).unwrap();

    // Full semi-annual period should be 1.0 under ISMA
    assert!((yf - 1.0).abs() < 1e-6);
}

#[test]
fn actact_isma_multiple_frequencies() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 4, 1);

    // Quarterly
    let ctx_q = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(3)),
        bus_basis: None,
    };
    let yf_q = DayCount::ActActIsma
        .year_fraction(start, end, ctx_q)
        .unwrap();

    // Monthly
    let ctx_m = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(1)),
        bus_basis: None,
    };
    let yf_m = DayCount::ActActIsma
        .year_fraction(start, end, ctx_m)
        .unwrap();

    // Different frequencies give different results for ISMA
    // Quarterly: 1 full period = 1.0
    assert!((yf_q - 1.0).abs() < 1.0e-6);
    // Monthly: 3 full periods = 3.0
    assert!((yf_m - 3.0).abs() < 1.0e-6);
}

#[test]
fn actact_isma_partial_period() {
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 4, 15);

    let ctx = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(6)),
        bus_basis: None,
    };

    let yf = DayCount::ActActIsma.year_fraction(start, end, ctx).unwrap();

    // 3 months out of 6-month period ≈ 0.5
    assert!(yf > 0.45 && yf < 0.55);
}

#[test]
fn actact_vs_actact_isma_comparison() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2026, 1, 1);

    let yf_isda = DayCount::ActAct
        .year_fraction(start, end, DayCountCtx::default())
        .unwrap();

    let ctx_isma = DayCountCtx {
        calendar: None,
        frequency: Some(Frequency::Months(12)),
        bus_basis: None,
    };
    let yf_isma = DayCount::ActActIsma
        .year_fraction(start, end, ctx_isma)
        .unwrap();

    // For a full year period with annual frequency, both should give 1.0
    assert!((yf_isda - 1.0).abs() < 1e-10);
    assert!((yf_isma - 1.0).abs() < 1e-10);
}

// =============================================================================
// Bus/252 Convention Tests - Tests calendar integration
// =============================================================================

#[test]
fn bus252_requires_calendar() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 1, 10);

    // Without calendar, should error
    let result = DayCount::Bus252.year_fraction(start, end, DayCountCtx::default());
    assert!(result.is_err());

    // With calendar, should work
    let calendar = TARGET2;
    let ctx = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };
    let yf = DayCount::Bus252.year_fraction(start, end, ctx).unwrap();
    assert!(yf > 0.0);
}

#[test]
fn bus252_counts_only_business_days() {
    let calendar = TARGET2;
    let start = make_date(2025, 1, 2); // Thursday
    let end = make_date(2025, 1, 6); // Monday (includes weekend)

    let ctx = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };

    let yf = DayCount::Bus252.year_fraction(start, end, ctx).unwrap();

    // Should count: Thu, Fri (skip Sat, Sun) = 2 business days
    let biz_days = yf * 252.0;
    assert!((biz_days - 2.0).abs() < 0.1);
}

#[test]
fn bus252_full_year_approximately_252() {
    let calendar = TARGET2;
    let start = make_date(2025, 1, 2);
    let end = make_date(2026, 1, 2);

    let ctx = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };

    let yf = DayCount::Bus252.year_fraction(start, end, ctx).unwrap();

    // Should be close to 1.0 (252 business days in a year)
    assert!((yf - 1.0).abs() < 0.05);
}

#[test]
fn bus252_excludes_holidays() {
    let calendar = TARGET2;

    // Period including Christmas
    let start = make_date(2024, 12, 23); // Monday
    let end = make_date(2024, 12, 30); // Monday next week

    let ctx = DayCountCtx {
        calendar: Some(&calendar),
        frequency: None,
        bus_basis: None,
    };

    let yf = DayCount::Bus252.year_fraction(start, end, ctx).unwrap();
    let biz_days = yf * 252.0;

    // Dec 25, 26 are holidays, plus weekend = only a few business days
    assert!(biz_days < 5.0);
}
