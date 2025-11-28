//! 30E/360 (Eurobond Basis) functional tests per ISDA 2006 Section 4.16(g)
//!
//! The 30E/360 convention differs from 30/360 US in how it handles day 31:
//! - D1=31 -> 30 (same as US)
//! - D2=31 -> 30 unconditionally (US only does this if D1 was adjusted to 30)
//!
//! This convention is standard for Eurobonds and international bonds.

use finstack_core::dates::{Date, DayCount, DayCountCtx};
use time::Month;

const TOL: f64 = 1e-12;

fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

#[test]
fn thirty_e360_d2_always_30_when_31() {
    // 30E/360: D2=31 -> 30 unconditionally (unlike US)
    // Jan 30 to Mar 31: D1=30, D2=30 (adjusted from 31)
    // Days: (30-30) + 60 = 60 days
    let yf = DayCount::ThirtyE360
        .year_fraction(d(2025, 1, 30), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 60.0 / 360.0).abs() < TOL,
        "Expected 60/360, got {}",
        yf
    );
}

#[test]
fn thirty_e360_vs_us_difference() {
    // This is the key test showing the difference between US and European conventions
    // US 30/360: Jan 30 to Mar 31 = D1=30 (not 31), D2=31 (stays, since D1 wasn't adjusted from 31)
    //   Actually D1=30 stays 30, D2=31 stays because D1_adj != 30 from 31 rule... 
    //   Wait, D1=30 so d1_adj = 30 (no change needed). D2=31, check if D1_adj == 30 -> yes!
    //   So US: D2 becomes 30. Days = (30-30) + 60 = 60
    //
    // Let me recalculate with different dates where they DO differ:
    // Jan 15 to Mar 31: D1=15, D2=31
    // US: D1_adj = 15 (not 31), D2=31 stays (D1_adj != 30) -> Days = (31-15) + 60 = 76
    // Euro: D1_adj = 15, D2=30 (always adjust 31) -> Days = (30-15) + 60 = 75
    let us = DayCount::Thirty360
        .year_fraction(d(2025, 1, 15), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();
    let euro = DayCount::ThirtyE360
        .year_fraction(d(2025, 1, 15), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();

    assert!(
        (us - 76.0 / 360.0).abs() < TOL,
        "US expected 76/360, got {}",
        us
    );
    assert!(
        (euro - 75.0 / 360.0).abs() < TOL,
        "Euro expected 75/360, got {}",
        euro
    );
    assert!(
        (us - euro - 1.0 / 360.0).abs() < TOL,
        "Difference should be 1 day"
    );
}

#[test]
fn thirty_e360_full_year() {
    // Full year should be 360/360 = 1.0
    let yf = DayCount::ThirtyE360
        .year_fraction(d(2025, 1, 1), d(2026, 1, 1), DayCountCtx::default())
        .unwrap();
    assert!((yf - 1.0).abs() < TOL, "Expected 1.0, got {}", yf);
}

#[test]
fn thirty_e360_jan31_to_mar31() {
    // D1=31 -> 30, D2=31 -> 30 (Euro always adjusts)
    // Days: (30-30) + 60 = 60 days
    let yf = DayCount::ThirtyE360
        .year_fraction(d(2025, 1, 31), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 60.0 / 360.0).abs() < TOL,
        "Expected 60/360, got {}",
        yf
    );
}

#[test]
fn thirty_e360_jan31_to_mar31_same_as_us() {
    // When D1=31 (adjusted to 30), both conventions give same result for D2=31
    // US: D1_adj=30, D2=31 -> D2=30 (because D1_adj==30)
    // Euro: D1_adj=30, D2=31 -> D2=30 (always)
    let us = DayCount::Thirty360
        .year_fraction(d(2025, 1, 31), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();
    let euro = DayCount::ThirtyE360
        .year_fraction(d(2025, 1, 31), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();

    assert!(
        (us - euro).abs() < TOL,
        "US and Euro should be equal when D1=31: US={}, Euro={}",
        us,
        euro
    );
}

#[test]
fn thirty_e360_feb28_not_adjusted() {
    // 30E/360 does NOT have Feb EOM rule like US does
    // Feb 28 stays as day 28
    // Feb 28 to Mar 31: D1=28, D2=30 (31 adjusted)
    // Days: (30-28) + 30 = 32 days
    let yf = DayCount::ThirtyE360
        .year_fraction(d(2025, 2, 28), d(2025, 3, 31), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 32.0 / 360.0).abs() < TOL,
        "Expected 32/360, got {}",
        yf
    );
}

#[test]
fn thirty_e360_short_period() {
    // Jan 15 to Jan 25 = 10 days
    let yf = DayCount::ThirtyE360
        .year_fraction(d(2025, 1, 15), d(2025, 1, 25), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 10.0 / 360.0).abs() < TOL,
        "Expected 10/360, got {}",
        yf
    );
}

#[test]
fn thirty_e360_dec31_to_jan31() {
    // Dec 31 to Jan 31 (next year)
    // D1=31 -> 30, D2=31 -> 30
    // Days: (30-30) + 30 = 30 days
    let yf = DayCount::ThirtyE360
        .year_fraction(d(2024, 12, 31), d(2025, 1, 31), DayCountCtx::default())
        .unwrap();
    assert!(
        (yf - 30.0 / 360.0).abs() < TOL,
        "Expected 30/360, got {}",
        yf
    );
}

