//! Schedule and cashflow generation tests for caps/floors.
//!
//! Validates period generation for multi-period caps and floors.

use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_valuations::cashflow::builder::schedule_utils::build_dates;
use time::macros::date;

#[test]
fn test_quarterly_schedule_generation() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2025 - 01 - 01);

    let schedule = build_dates(
        start,
        end,
        Frequency::quarterly(),
        StubKind::None,
        BusinessDayConvention::ModifiedFollowing,
        None,
    );

    // Should have 5 dates (4 periods): Jan, Apr, Jul, Oct, Jan
    assert_eq!(schedule.dates.len(), 5, "Should have 5 quarterly dates");
    assert_eq!(schedule.dates[0], start);
    assert_eq!(*schedule.dates.last().unwrap(), end);
}

#[test]
fn test_semi_annual_schedule() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2026 - 01 - 01);

    let schedule = build_dates(
        start,
        end,
        Frequency::semi_annual(),
        StubKind::None,
        BusinessDayConvention::Following,
        None,
    );

    // 2 years semi-annual = 5 dates (4 periods): Jan, Jul, Jan, Jul, Jan
    assert_eq!(schedule.dates.len(), 5, "Should have 5 semi-annual dates");
}

#[test]
fn test_annual_schedule() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let schedule = build_dates(
        start,
        end,
        Frequency::annual(),
        StubKind::None,
        BusinessDayConvention::Following,
        None,
    );

    // 5 years annual = 6 dates (5 periods)
    assert_eq!(schedule.dates.len(), 6, "Should have 6 annual dates");
}

#[test]
fn test_monthly_schedule() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2024 - 07 - 01);

    let schedule = build_dates(
        start,
        end,
        Frequency::monthly(),
        StubKind::None,
        BusinessDayConvention::Following,
        None,
    );

    // 6 months = 7 dates (6 periods)
    assert_eq!(schedule.dates.len(), 7, "Should have 7 monthly dates");
}

#[test]
fn test_schedule_ordering() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2025 - 01 - 01);

    let schedule = build_dates(
        start,
        end,
        Frequency::quarterly(),
        StubKind::None,
        BusinessDayConvention::Following,
        None,
    );

    // Verify dates are in ascending order
    for i in 1..schedule.dates.len() {
        assert!(
            schedule.dates[i] > schedule.dates[i - 1],
            "Dates should be in ascending order"
        );
    }
}

#[test]
fn test_period_coverage() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2025 - 01 - 01);

    let schedule = build_dates(
        start,
        end,
        Frequency::quarterly(),
        StubKind::None,
        BusinessDayConvention::Following,
        None,
    );

    // First date should equal start
    assert_eq!(schedule.dates[0], start, "First date should equal start");

    // Last date should equal end
    assert_eq!(
        *schedule.dates.last().unwrap(),
        end,
        "Last date should equal end"
    );
}

#[test]
fn test_year_fraction_calculation() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2024 - 04 - 01);

    let day_count = DayCount::Act360;
    let yf = day_count
        .year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
        .unwrap();

    // 91 days / 360 = 0.2527...
    assert!(
        yf > 0.25 && yf < 0.26,
        "Year fraction should be ~0.25: {}",
        yf
    );
}

#[test]
fn test_different_day_count_conventions() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2024 - 07 - 01);

    let act360 = DayCount::Act360
        .year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
        .unwrap();
    let act365 = DayCount::Act365F
        .year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
        .unwrap();
    let thirty_360 = DayCount::Thirty360
        .year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
        .unwrap();

    // Different day counts should produce different results
    assert!(act360 != act365, "ACT/360 should differ from ACT/365F");
    assert!(act360 != thirty_360, "ACT/360 should differ from 30/360");
}

#[test]
fn test_leap_year_handling() {
    let start = date!(2024 - 02 - 28);
    let end = date!(2024 - 03 - 01); // 2024 is a leap year

    let day_count = DayCount::Act365F;
    let yf = day_count
        .year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
        .unwrap();

    // Should account for Feb 29 - 2 days in a 366-day year
    assert!(yf > 0.0, "Should handle leap year: {}", yf);
    assert!(yf < 0.01, "Two days should be small fraction: {}", yf);
}
