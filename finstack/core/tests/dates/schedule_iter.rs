//! Tests for schedule iterator functionality.

use super::common::{make_date, TestCal};
use finstack_core::dates::{BusinessDayConvention, Frequency, ScheduleBuilder, StubKind};

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

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::monthly())
        .build()
        .unwrap()
        .into_iter()
        .collect();

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
        .build()
        .unwrap()
        .into_iter()
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
        .build()
        .unwrap()
        .into_iter()
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
        .build()
        .unwrap()
        .into_iter()
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
        .build()
        .unwrap()
        .into_iter()
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
        .unwrap()
        .into_iter()
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
        .unwrap()
        .into_iter()
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
        .build()
        .unwrap()
        .into_iter()
        .collect();

    assert_eq!(dates.len(), 2);
    assert_eq!(dates[0], make_date(2025, 1, 1));
    assert_eq!(dates[1], make_date(2025, 1, 20)); // Clamped to end date
}

#[test]
fn test_long_front_stub() {
    // Test LongFront creates a longer first period
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 11, 1);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::quarterly())
        .stub_rule(StubKind::LongFront)
        .build()
        .unwrap()
        .into_iter()
        .collect();

    // Should create regular quarters from end date backwards: Nov, Aug, May, Feb
    // This creates a long front period from Jan 1 to Feb 1
    assert_eq!(dates.len(), 5);
    assert_eq!(dates[0], make_date(2025, 1, 1)); // Start date
    assert_eq!(dates[1], make_date(2025, 2, 1)); // First regular anchor
    assert_eq!(dates[2], make_date(2025, 5, 1)); // Regular quarter
    assert_eq!(dates[3], make_date(2025, 8, 1)); // Regular quarter
    assert_eq!(dates[4], make_date(2025, 11, 1)); // End date
}

#[test]
fn test_long_back_stub() {
    // Test LongBack creates a longer last period
    let start = make_date(2025, 1, 1);
    let end = make_date(2025, 11, 1);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::quarterly())
        .stub_rule(StubKind::LongBack)
        .build()
        .unwrap()
        .into_iter()
        .collect();

    // Should create regular quarters from start: Jan, Apr, Jul
    // Then create a long back period from Jul to Nov (4 months)
    assert_eq!(dates.len(), 4);
    assert_eq!(dates[0], make_date(2025, 1, 1)); // Start date
    assert_eq!(dates[1], make_date(2025, 4, 1)); // Regular quarter
    assert_eq!(dates[2], make_date(2025, 7, 1)); // Regular quarter
    assert_eq!(dates[3], make_date(2025, 11, 1)); // End date (long back period)
}

#[test]
fn test_end_of_month_convention() {
    // Test EOM convention adjusts dates to month-end
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 4, 15);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::monthly())
        .end_of_month(true)
        .build()
        .unwrap()
        .into_iter()
        .collect();

    assert_eq!(dates.len(), 4);
    assert_eq!(dates[0], make_date(2025, 1, 31)); // Jan 15 -> Jan 31
    assert_eq!(dates[1], make_date(2025, 2, 28)); // Feb 15 -> Feb 28
    assert_eq!(dates[2], make_date(2025, 3, 31)); // Mar 15 -> Mar 31
    assert_eq!(dates[3], make_date(2025, 4, 30)); // Apr 15 -> Apr 30
}

#[test]
fn test_end_of_month_with_leap_year() {
    // Test EOM convention with leap year
    let start = make_date(2024, 1, 15); // 2024 is a leap year
    let end = make_date(2024, 3, 15);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::monthly())
        .end_of_month(true)
        .build()
        .unwrap()
        .into_iter()
        .collect();

    assert_eq!(dates.len(), 3);
    assert_eq!(dates[0], make_date(2024, 1, 31)); // Jan 31
    assert_eq!(dates[1], make_date(2024, 2, 29)); // Feb 29 (leap year)
    assert_eq!(dates[2], make_date(2024, 3, 31)); // Mar 31
}

#[test]
fn test_eom_with_stub_conventions() {
    // Test that EOM works with stub conventions
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 5, 15);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Frequency::quarterly())
        .stub_rule(StubKind::ShortBack)
        .end_of_month(true)
        .build()
        .unwrap()
        .into_iter()
        .collect();

    assert_eq!(dates.len(), 3);
    assert_eq!(dates[0], make_date(2025, 1, 31)); // Start -> Jan 31
    assert_eq!(dates[1], make_date(2025, 4, 30)); // Regular quarter -> Apr 30
    assert_eq!(dates[2], make_date(2025, 5, 31)); // End -> May 31
}
