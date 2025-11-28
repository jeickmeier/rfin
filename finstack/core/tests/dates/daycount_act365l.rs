//! Act/365L (AFB) functional tests
//!
//! Act/365L uses 366 as denominator if February 29 falls between start (exclusive)
//! and end (inclusive), otherwise uses 365.
//!
//! This convention is used in French markets and some bond calculations.

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use time::Month;

const TOL: f64 = 1e-12;

fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

#[test]
fn act365l_period_contains_feb29_uses_366() {
    // Jan 1 to Mar 1 in 2024 (leap year) = 60 actual days, contains Feb 29
    let yf = DayCount::Act365L
        .year_fraction(d(2024, 1, 1), d(2024, 3, 1), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 60.0 / 366.0).abs() < TOL,
        "Expected 60/366, got {}",
        yf
    );
}

#[test]
fn act365l_period_no_feb29_uses_365() {
    // Jan 1 to Mar 1 in 2025 (non-leap) = 59 actual days, no Feb 29
    let yf = DayCount::Act365L
        .year_fraction(d(2025, 1, 1), d(2025, 3, 1), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 59.0 / 365.0).abs() < TOL,
        "Expected 59/365, got {}",
        yf
    );
}

#[test]
fn act365l_period_ending_on_feb29() {
    // Period ending exactly on Feb 29 (inclusive)
    // 59 actual days (Jan 31 + Feb 1-29), Feb 29 in range -> 366 denom
    let yf = DayCount::Act365L
        .year_fraction(d(2024, 1, 1), d(2024, 2, 29), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 59.0 / 366.0).abs() < TOL,
        "Expected 59/366, got {}",
        yf
    );
}

#[test]
fn act365l_period_before_feb29_in_leap_year() {
    // Jan 1 to Feb 28 in 2024 - does NOT contain Feb 29 (end is exclusive of Feb 29)
    // 58 actual days, Feb 29 NOT in [Jan 1, Feb 28] -> 365 denom
    let yf = DayCount::Act365L
        .year_fraction(d(2024, 1, 1), d(2024, 2, 28), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 58.0 / 365.0).abs() < TOL,
        "Expected 58/365, got {}",
        yf
    );
}

#[test]
fn act365l_full_year_leap() {
    // Full leap year: 366 days / 366 = 1.0
    let yf = DayCount::Act365L
        .year_fraction(d(2024, 1, 1), d(2025, 1, 1), DayCountCtx::default())
        .unwrap();
    assert!((yf - 1.0).abs() < TOL, "Expected 1.0, got {}", yf);
}

#[test]
fn act365l_full_year_non_leap() {
    // Full non-leap year: 365 days / 365 = 1.0
    let yf = DayCount::Act365L
        .year_fraction(d(2025, 1, 1), d(2026, 1, 1), DayCountCtx::default())
        .unwrap();
    assert!((yf - 1.0).abs() < TOL, "Expected 1.0, got {}", yf);
}

#[test]
fn act365l_spanning_leap_year_boundary() {
    // Dec 1, 2023 to Mar 1, 2024 - spans into leap year, contains Feb 29
    // 91 actual days, contains Feb 29 -> 366 denom
    let yf = DayCount::Act365L
        .year_fraction(d(2023, 12, 1), d(2024, 3, 1), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 91.0 / 366.0).abs() < TOL,
        "Expected 91/366, got {}",
        yf
    );
}

#[test]
fn act365l_spanning_non_leap_year_boundary() {
    // Dec 1, 2024 to Mar 1, 2025 - spans year boundary, 2025 not leap
    // 90 actual days, no Feb 29 in range -> 365 denom
    let yf = DayCount::Act365L
        .year_fraction(d(2024, 12, 1), d(2025, 3, 1), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 90.0 / 365.0).abs() < TOL,
        "Expected 90/365, got {}",
        yf
    );
}

#[test]
fn act365l_single_day_feb29() {
    // Feb 28 to Feb 29 = 1 day, contains Feb 29
    let yf = DayCount::Act365L
        .year_fraction(d(2024, 2, 28), d(2024, 2, 29), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 1.0 / 366.0).abs() < TOL,
        "Expected 1/366, got {}",
        yf
    );
}

#[test]
fn act365l_single_day_not_feb29() {
    // Mar 1 to Mar 2 = 1 day, no Feb 29
    let yf = DayCount::Act365L
        .year_fraction(d(2024, 3, 1), d(2024, 3, 2), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 1.0 / 365.0).abs() < TOL,
        "Expected 1/365, got {}",
        yf
    );
}

