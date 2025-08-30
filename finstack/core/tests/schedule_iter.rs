//! Tests for schedule iterator functionality.

mod common;

use common::{make_date, TestCal};
use finstack_core::dates::{schedule, BusinessDayConvention, Frequency, ScheduleBuilder, StubKind};

#[test]
fn test_frequency_constructors() {
    // Test all frequency constructors
    assert_eq!(Frequency::annual().months(), Some(12));
    assert_eq!(Frequency::semi_annual().months(), Some(6));
    assert_eq!(Frequency::quarterly().months(), Some(3));
    assert_eq!(Frequency::monthly().months(), Some(1));

    assert_eq!(Frequency::daily().days(), Some(1));
    assert_eq!(Frequency::weekly().days(), Some(7));
    assert_eq!(Frequency::biweekly().days(), Some(14));

    // Test that months() returns None for day-based frequencies
    assert_eq!(Frequency::daily().months(), None);
    assert_eq!(Frequency::weekly().months(), None);

    // Test that days() returns None for month-based frequencies
    assert_eq!(Frequency::monthly().days(), None);
    assert_eq!(Frequency::quarterly().days(), None);
}

#[test]
fn test_basic_schedule() {
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 4, 15);

    let dates: Vec<_> = schedule(start, end, Frequency::monthly()).collect();

    assert_eq!(dates.len(), 4);
    assert_eq!(dates[0], make_date(2025, 1, 15));
    assert_eq!(dates[1], make_date(2025, 2, 15));
    assert_eq!(dates[2], make_date(2025, 3, 15));
    assert_eq!(dates[3], make_date(2025, 4, 15));
}

#[test]
fn test_quarterly_schedule_with_short_back_stub() {
    // Period not evenly divisible by quarterly frequency
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 11, 1); // 10 months

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::quarterly())
        .stub_rule(StubKind::None) // Default behavior creates short back stub
        .build_raw()
        .collect();

    // Should get: Jan, Apr, Jul, Oct, Nov (short stub at end)
    assert_eq!(dates.len(), 5);
    assert_eq!(dates[0], make_date(2025, 1, 1));
    assert_eq!(dates[1], make_date(2025, 4, 1));
    assert_eq!(dates[2], make_date(2025, 7, 1));
    assert_eq!(dates[3], make_date(2025, 10, 1));
    assert_eq!(dates[4], make_date(2025, 11, 1));
}

#[test]
fn test_short_front_stub() {
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 11, 1);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::quarterly())
        .stub_rule(StubKind::ShortFront)
        .build_raw()
        .collect();

    // Should get: Jan, Feb, May, Aug, Nov (short stub at front)
    assert_eq!(dates.len(), 5);
    assert_eq!(dates[0], make_date(2025, 1, 1));
    assert_eq!(dates[1], make_date(2025, 2, 1));
    assert_eq!(dates[2], make_date(2025, 5, 1));
    assert_eq!(dates[3], make_date(2025, 8, 1));
    assert_eq!(dates[4], make_date(2025, 11, 1));
}

#[test]
fn test_day_based_frequency() {
    let start = make_date(2025, 1, 1); // Wednesday
    let end = make_date(2025, 1, 15);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::weekly()) // 7 days
        .build_raw()
        .collect();

    assert_eq!(dates.len(), 3);
    assert_eq!(dates[0], make_date(2025, 1, 1)); // Wed
    assert_eq!(dates[1], make_date(2025, 1, 8)); // Wed + 7 days
    assert_eq!(dates[2], make_date(2025, 1, 15)); // Wed + 14 days
}

#[test]
fn test_single_date_schedule() {
    let date = make_date(2025, 1, 15);

    let dates: Vec<_> = ScheduleBuilder::new(date, date)
        .frequency(Frequency::monthly())
        .build_raw()
        .collect();

    assert_eq!(dates.len(), 1);
    assert_eq!(dates[0], date);
}

#[test]
fn test_schedule_with_business_day_adjustment() {
    let cal = TestCal::new().with_holiday(make_date(2025, 1, 1)); // New Year's Day (Wednesday)

    let start = make_date(2025, 1, 1); // Holiday Wednesday
    let end = make_date(2025, 1, 8);

    // Test that the builder can handle adjustment (even if AdjustIter is not public)
    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::weekly())
        .adjust_with(BusinessDayConvention::Following, &cal)
        .build()
        .collect();

    // First date should be adjusted from Jan 1 (holiday) to Jan 2
    assert_eq!(dates.len(), 2);
    assert_eq!(dates[0], make_date(2025, 1, 2)); // Thursday (adjusted from holiday)
    assert_eq!(dates[1], make_date(2025, 1, 8)); // Wednesday
}

#[test]
fn test_schedule_builder_with_adjustment() {
    let cal = TestCal::new().with_holiday(make_date(2025, 1, 1)); // New Year's Day

    let start = make_date(2025, 1, 1); // Wednesday (holiday)
    let end = make_date(2025, 3, 1);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::monthly())
        .adjust_with(BusinessDayConvention::Following, &cal)
        .build()
        .collect();

    // First date should be adjusted from Jan 1 (holiday) to Jan 2
    assert_eq!(dates[0], make_date(2025, 1, 2)); // Thursday
    assert_eq!(dates[1], make_date(2025, 2, 3)); // Saturday -> Monday Feb 3
    assert_eq!(dates[2], make_date(2025, 3, 3)); // Saturday -> Monday Mar 3
}

#[test]
fn test_uneven_period_clamping() {
    // Test that when step would overshoot end date, it clamps to end date
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 1, 20); // Not a multiple of monthly frequency

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::monthly())
        .build_raw()
        .collect();

    assert_eq!(dates.len(), 2);
    assert_eq!(dates[0], make_date(2025, 1, 1));
    assert_eq!(dates[1], make_date(2025, 1, 20)); // Clamped to end date
}
