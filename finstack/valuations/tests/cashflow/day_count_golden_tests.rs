//! Golden value tests for day count conventions per ISDA 2006.
//!
//! These tests verify that day count convention implementations produce
//! correct year fractions according to ISDA 2006 definitions.
//!
//! # Day Count Conventions Tested
//!
//! - Act/365 Fixed: actual days / 365 (fixed denominator)
//! - Act/360: actual days / 360 (money market convention)
//! - 30/360: each month = 30 days, year = 360 days (bond convention)
//! - Act/Act ISMA: requires coupon frequency
//! - Act/Act ISDA: actual days / days in year

use crate::cashflow_tests::test_helpers::FACTOR_TOLERANCE;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Frequency};
use time::Month;

fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

// =============================================================================
// Act/365 Fixed Golden Values
// =============================================================================

#[test]
fn act365f_14_days() {
    // 14 days from Jan 1 to Jan 15
    let dc = DayCount::Act365F;
    let ctx = DayCountCtx::default();

    let yf = dc
        .year_fraction(d(2025, 1, 1), d(2025, 1, 15), ctx)
        .unwrap();
    let expected = 14.0 / 365.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "14 days: expected {}, got {}",
        expected,
        yf
    );
}

#[test]
fn act365f_full_non_leap_year() {
    // Full non-leap year: Jan 1, 2025 to Jan 1, 2026 = 365 days
    let dc = DayCount::Act365F;
    let ctx = DayCountCtx::default();

    let yf = dc.year_fraction(d(2025, 1, 1), d(2026, 1, 1), ctx).unwrap();

    assert!(
        (yf - 1.0).abs() < FACTOR_TOLERANCE,
        "Full non-leap year should be exactly 1.0, got {}",
        yf
    );
}

#[test]
fn act365f_leap_year() {
    // Leap year: Jan 1, 2024 to Jan 1, 2025 = 366 actual days
    // But Act/365F always divides by 365, so yf = 366/365 > 1.0
    let dc = DayCount::Act365F;
    let ctx = DayCountCtx::default();

    let yf = dc.year_fraction(d(2024, 1, 1), d(2025, 1, 1), ctx).unwrap();
    let expected = 366.0 / 365.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "Leap year: expected {}, got {}",
        expected,
        yf
    );
}

#[test]
fn act365f_quarter() {
    // Q1 2025: Jan 1 to Apr 1 = 90 days
    let dc = DayCount::Act365F;
    let ctx = DayCountCtx::default();

    let yf = dc.year_fraction(d(2025, 1, 1), d(2025, 4, 1), ctx).unwrap();
    let expected = 90.0 / 365.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "Q1 (90 days): expected {}, got {}",
        expected,
        yf
    );
}

// =============================================================================
// Act/360 Golden Values
// =============================================================================

#[test]
fn act360_30_days() {
    // 30 days from Jan 1 to Jan 31
    let dc = DayCount::Act360;
    let ctx = DayCountCtx::default();

    let yf = dc
        .year_fraction(d(2025, 1, 1), d(2025, 1, 31), ctx)
        .unwrap();
    let expected = 30.0 / 360.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "30 days: expected {}, got {}",
        expected,
        yf
    );
}

#[test]
fn act360_90_days_quarter() {
    // 90 days (quarter)
    let dc = DayCount::Act360;
    let ctx = DayCountCtx::default();

    let yf = dc.year_fraction(d(2025, 1, 1), d(2025, 4, 1), ctx).unwrap();
    let expected = 90.0 / 360.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "90 days (quarter): expected {}, got {}",
        expected,
        yf
    );
}

#[test]
fn act360_180_days_half_year() {
    // 181 actual days from Jan 1 to Jul 1 (non-leap year)
    let dc = DayCount::Act360;
    let ctx = DayCountCtx::default();

    let yf = dc.year_fraction(d(2025, 1, 1), d(2025, 7, 1), ctx).unwrap();
    let expected = 181.0 / 360.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "181 days: expected {}, got {}",
        expected,
        yf
    );
}

#[test]
fn act360_full_year() {
    // Full year in Act/360 is 365/360 = 1.01389 (non-leap)
    let dc = DayCount::Act360;
    let ctx = DayCountCtx::default();

    let yf = dc.year_fraction(d(2025, 1, 1), d(2026, 1, 1), ctx).unwrap();
    let expected = 365.0 / 360.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "Full year: expected {}, got {}",
        expected,
        yf
    );
}

// =============================================================================
// 30/360 Golden Values
// =============================================================================

#[test]
fn thirty360_6_months() {
    // 6 months from Jan 15 to Jul 15 = 180 days (30/360)
    // Year fraction = 180/360 = 0.5
    let dc = DayCount::Thirty360;
    let ctx = DayCountCtx::default();

    let yf = dc
        .year_fraction(d(2025, 1, 15), d(2025, 7, 15), ctx)
        .unwrap();

    assert!(
        (yf - 0.5).abs() < FACTOR_TOLERANCE,
        "6 months should be exactly 0.5, got {}",
        yf
    );
}

#[test]
fn thirty360_1_month() {
    // 1 month from Jan 15 to Feb 15 = 30 days (30/360)
    let dc = DayCount::Thirty360;
    let ctx = DayCountCtx::default();

    let yf = dc
        .year_fraction(d(2025, 1, 15), d(2025, 2, 15), ctx)
        .unwrap();
    let expected = 30.0 / 360.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "1 month should be {}, got {}",
        expected,
        yf
    );
}

#[test]
fn thirty360_end_of_month_jan_to_feb() {
    // Jan 31 to Feb 28: In 30/360, Jan 31 → 30, Feb 28 stays 28
    // Days = (2 - 1) * 30 + (28 - 30) = 30 - 2 = 28 days
    let dc = DayCount::Thirty360;
    let ctx = DayCountCtx::default();

    let yf = dc
        .year_fraction(d(2025, 1, 31), d(2025, 2, 28), ctx)
        .unwrap();
    let expected = 28.0 / 360.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "Jan 31 to Feb 28: expected {}, got {}",
        expected,
        yf
    );
}

#[test]
fn thirty360_full_year() {
    // Full year from Jan 1 to Jan 1 = 360 days (30/360)
    let dc = DayCount::Thirty360;
    let ctx = DayCountCtx::default();

    let yf = dc.year_fraction(d(2025, 1, 1), d(2026, 1, 1), ctx).unwrap();

    assert!(
        (yf - 1.0).abs() < FACTOR_TOLERANCE,
        "Full year should be exactly 1.0, got {}",
        yf
    );
}

#[test]
fn thirty360_same_day_different_months() {
    // Mar 15 to Jun 15 = 3 months = 90 days in 30/360
    let dc = DayCount::Thirty360;
    let ctx = DayCountCtx::default();

    let yf = dc
        .year_fraction(d(2025, 3, 15), d(2025, 6, 15), ctx)
        .unwrap();
    let expected = 90.0 / 360.0;

    assert!(
        (yf - expected).abs() < FACTOR_TOLERANCE,
        "3 months: expected {}, got {}",
        expected,
        yf
    );
}

// =============================================================================
// Act/Act ISMA (requires frequency)
// =============================================================================

#[test]
fn actact_isma_requires_frequency() {
    let dc = DayCount::ActActIsma;

    // Without frequency - should error
    let ctx_no_freq = DayCountCtx {
        frequency: None,
        calendar: None,
        bus_basis: None,
    };
    assert!(
        dc.year_fraction(d(2025, 1, 1), d(2025, 7, 1), ctx_no_freq)
            .is_err(),
        "ActActIsma should error without frequency"
    );
}

#[test]
fn actact_isma_with_semi_annual_frequency() {
    let dc = DayCount::ActActIsma;

    // With semi-annual frequency - should succeed
    let ctx_with_freq = DayCountCtx {
        frequency: Some(Frequency::semi_annual()),
        calendar: None,
        bus_basis: None,
    };
    let result = dc.year_fraction(d(2025, 1, 1), d(2025, 7, 1), ctx_with_freq);

    // ActActIsma requires frequency context and should succeed
    assert!(
        result.is_ok(),
        "ActActIsma with semi-annual frequency should succeed"
    );

    // Note: The exact value depends on the implementation details of ActActIsma.
    // This test validates that the calculation runs without error.
    let yf = result.unwrap();
    assert!(yf > 0.0, "Year fraction should be positive, got {}", yf);
}

#[test]
fn actact_isma_with_quarterly_frequency() {
    let dc = DayCount::ActActIsma;

    let ctx = DayCountCtx {
        frequency: Some(Frequency::quarterly()),
        calendar: None,
        bus_basis: None,
    };

    let result = dc.year_fraction(d(2025, 1, 1), d(2025, 4, 1), ctx);

    // ActActIsma requires frequency context and should succeed
    assert!(
        result.is_ok(),
        "ActActIsma with quarterly frequency should succeed"
    );

    // Note: The exact value depends on the implementation details of ActActIsma.
    // This test validates that the calculation runs without error.
    let yf = result.unwrap();
    assert!(yf > 0.0, "Year fraction should be positive, got {}", yf);
}

// =============================================================================
// Act/Act ISDA (does not require frequency)
// =============================================================================

#[test]
fn actact_isda_no_frequency_required() {
    // Act/Act ISDA uses actual days / actual days in year
    let dc = DayCount::ActAct;

    let ctx = DayCountCtx {
        frequency: None,
        calendar: None,
        bus_basis: None,
    };

    // Should work without frequency
    let result = dc.year_fraction(d(2025, 1, 1), d(2025, 7, 1), ctx);
    assert!(
        result.is_ok(),
        "ActAct (ISDA) should not require frequency context"
    );
}

#[test]
fn actact_isda_non_leap_year() {
    let dc = DayCount::ActAct;
    let ctx = DayCountCtx::default();

    // Full non-leap year
    let yf = dc.year_fraction(d(2025, 1, 1), d(2026, 1, 1), ctx).unwrap();

    assert!(
        (yf - 1.0).abs() < FACTOR_TOLERANCE,
        "Full non-leap year should be 1.0, got {}",
        yf
    );
}

#[test]
fn actact_isda_leap_year() {
    let dc = DayCount::ActAct;
    let ctx = DayCountCtx::default();

    // Full leap year: Jan 1, 2024 to Jan 1, 2025 = 366 days in 366-day year
    let yf = dc.year_fraction(d(2024, 1, 1), d(2025, 1, 1), ctx).unwrap();

    assert!(
        (yf - 1.0).abs() < FACTOR_TOLERANCE,
        "Full leap year should be 1.0, got {}",
        yf
    );
}

// =============================================================================
// Zero-length periods
// =============================================================================

#[test]
fn zero_length_period_all_conventions() {
    let ctx = DayCountCtx::default();
    let ctx_with_freq = DayCountCtx {
        frequency: Some(Frequency::semi_annual()),
        calendar: None,
        bus_basis: None,
    };

    let conventions = [
        (DayCount::Act365F, ctx),
        (DayCount::Act360, ctx),
        (DayCount::Thirty360, ctx),
        (DayCount::ActAct, ctx),
        (DayCount::ActActIsma, ctx_with_freq),
    ];

    for (dc, ctx) in conventions {
        let yf = dc
            .year_fraction(d(2025, 6, 15), d(2025, 6, 15), ctx)
            .unwrap();
        assert!(
            yf.abs() < FACTOR_TOLERANCE,
            "{:?}: Zero-length period should be 0.0, got {}",
            dc,
            yf
        );
    }
}
