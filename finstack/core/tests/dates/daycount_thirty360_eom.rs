//! 30/360 End-of-Month tests per ISDA 2006 Section 4.16(f)
//!
//! These tests verify correct handling of February end-of-month dates
//! in the 30/360 US (Bond Basis) convention.

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use time::Month;

const TOL: f64 = 1e-12;

fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

#[test]
fn thirty360_us_feb28_to_mar31_non_leap() {
    // ISDA: Feb 28 (last day of Feb in non-leap year) -> treat as day 30
    // Mar 31 -> treat as day 30 (since D1 was adjusted to 30)
    // Result: (30 - 30) + 30 = 30 days
    let yf = DayCount::Thirty360
        .year_fraction(d(2025, 2, 28), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 30.0 / 360.0).abs() < TOL,
        "Expected 30/360, got {}",
        yf
    );
}

#[test]
fn thirty360_us_feb29_to_mar31_leap() {
    // Leap year: Feb 29 is last day of Feb -> treat as day 30
    // Mar 31 -> treat as day 30 (since D1 was adjusted to 30)
    // Result: (30 - 30) + 30 = 30 days
    let yf = DayCount::Thirty360
        .year_fraction(d(2024, 2, 29), d(2024, 3, 31), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 30.0 / 360.0).abs() < TOL,
        "Expected 30/360, got {}",
        yf
    );
}

#[test]
fn thirty360_us_feb27_to_mar31() {
    // Feb 27 is NOT last day of Feb -> D1 stays 27
    // Mar 31 -> stays 31 (D1 was not 30)
    // Result: (31 - 27) + 30 = 34 days
    let yf = DayCount::Thirty360
        .year_fraction(d(2025, 2, 27), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 34.0 / 360.0).abs() < TOL,
        "Expected 34/360, got {}",
        yf
    );
}

#[test]
fn thirty360_us_feb28_to_feb28_next_year() {
    // Both are last day of Feb (non-leap 2024 to non-leap 2025)
    // D1 = 30, D2 = 30 (both adjusted)
    // Result: 360 days = 1.0
    let yf = DayCount::Thirty360
        .year_fraction(d(2024, 2, 28), d(2025, 2, 28), DayCountCtx::default())
        .unwrap();
    assert!((yf - 1.0).abs() < TOL, "Expected 1.0, got {}", yf);
}

#[test]
fn thirty360_us_feb29_to_feb28_next_year() {
    // Feb 29 (leap) to Feb 28 (non-leap next year)
    // Both are last day of Feb -> both adjusted to 30
    // Result: 360 days = 1.0
    let yf = DayCount::Thirty360
        .year_fraction(d(2024, 2, 29), d(2025, 2, 28), DayCountCtx::default())
        .unwrap();
    assert!((yf - 1.0).abs() < TOL, "Expected 1.0, got {}", yf);
}

#[test]
fn thirty360_us_jan31_to_feb28() {
    // Jan 31 -> D1 = 30 (31 rule)
    // Feb 28 (last day) -> but D1 was not Feb EOM, so we check D2 EOM rule
    // Result depends on whether D1 was Feb EOM (it wasn't)
    // So D2 stays 28
    // Days: (28 - 30) + 30 = 28 days
    let yf = DayCount::Thirty360
        .year_fraction(d(2025, 1, 31), d(2025, 2, 28), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 28.0 / 360.0).abs() < TOL,
        "Expected 28/360, got {}",
        yf
    );
}

#[test]
fn thirty360_us_feb28_to_mar30() {
    // Feb 28 (EOM) -> D1 = 30
    // Mar 30 -> D2 stays 30 (not 31)
    // Days: (30 - 30) + 30 = 30 days
    let yf = DayCount::Thirty360
        .year_fraction(d(2025, 2, 28), d(2025, 3, 30), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 30.0 / 360.0).abs() < TOL,
        "Expected 30/360, got {}",
        yf
    );
}

#[test]
fn thirty360_us_feb28_to_apr30() {
    // Feb 28 (EOM) -> D1 = 30
    // Apr 30 -> D2 stays 30
    // Days: (30 - 30) + 60 = 60 days
    let yf = DayCount::Thirty360
        .year_fraction(d(2025, 2, 28), d(2025, 4, 30), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 60.0 / 360.0).abs() < TOL,
        "Expected 60/360, got {}",
        yf
    );
}

