//! Tests for schedule iterator functionality.

use super::common::{make_date, TestCal};
use finstack_core::dates::{BusinessDayConvention, ScheduleBuilder, StubKind, Tenor};

#[test]
fn test_basic_schedule() {
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 4, 15);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Tenor::monthly())
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
        .frequency(Tenor::quarterly())
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
        .frequency(Tenor::quarterly())
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
        .frequency(Tenor::weekly()) // 7 days
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
        .frequency(Tenor::monthly())
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
        .frequency(Tenor::weekly())
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
        .frequency(Tenor::monthly())
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
        .frequency(Tenor::monthly())
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
        .frequency(Tenor::quarterly())
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
        .frequency(Tenor::quarterly())
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
        .frequency(Tenor::monthly())
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
        .frequency(Tenor::monthly())
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
        .frequency(Tenor::quarterly())
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

#[test]
fn test_eom_jan30_roll_to_feb_leap_year() {
    // EOM: Jan 30 should roll to month-end including Feb 29 in leap year
    let start = make_date(2024, 1, 30);
    let end = make_date(2024, 3, 30);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Tenor::monthly())
        .end_of_month(true)
        .build()
        .unwrap()
        .into_iter()
        .collect();

    assert_eq!(dates.len(), 3);
    assert_eq!(dates[0], make_date(2024, 1, 31)); // Jan 30 -> 31
    assert_eq!(dates[1], make_date(2024, 2, 29)); // Feb -> 29 (leap)
    assert_eq!(dates[2], make_date(2024, 3, 31)); // Mar -> 31
}

#[test]
fn test_eom_jan30_roll_to_feb_non_leap() {
    // EOM: Jan 30 should roll to month-end, Feb 28 in non-leap year
    let start = make_date(2025, 1, 30);
    let end = make_date(2025, 3, 30);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Tenor::monthly())
        .end_of_month(true)
        .build()
        .unwrap()
        .into_iter()
        .collect();

    assert_eq!(dates.len(), 3);
    assert_eq!(dates[0], make_date(2025, 1, 31)); // Jan 30 -> 31
    assert_eq!(dates[1], make_date(2025, 2, 28)); // Feb -> 28 (non-leap)
    assert_eq!(dates[2], make_date(2025, 3, 31)); // Mar -> 31
}

#[test]
fn test_eom_quarterly_through_feb() {
    // Quarterly schedule starting Jan 31 through May should handle Feb correctly
    let start = make_date(2024, 1, 31);
    let end = make_date(2024, 7, 31);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .frequency(Tenor::quarterly())
        .end_of_month(true)
        .build()
        .unwrap()
        .into_iter()
        .collect();

    assert_eq!(dates.len(), 3);
    assert_eq!(dates[0], make_date(2024, 1, 31)); // Jan 31
    assert_eq!(dates[1], make_date(2024, 4, 30)); // Apr 30 (not 31)
    assert_eq!(dates[2], make_date(2024, 7, 31)); // Jul 31
}

// ============================================================================
// IMM Schedule Tests
// ============================================================================

#[test]
fn test_imm_schedule_basic() {
    // Standard IMM schedule: third Wednesday of Mar/Jun/Sep/Dec
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 12, 31);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .imm()
        .build()
        .unwrap()
        .into_iter()
        .collect();

    // Should get 4 dates: Mar 19, Jun 18, Sep 17, Dec 17 (2025 third Wednesdays)
    assert_eq!(dates.len(), 4);
    assert_eq!(dates[0], make_date(2025, 3, 19)); // Third Wednesday of March
    assert_eq!(dates[1], make_date(2025, 6, 18)); // Third Wednesday of June
    assert_eq!(dates[2], make_date(2025, 9, 17)); // Third Wednesday of September
    assert_eq!(dates[3], make_date(2025, 12, 17)); // Third Wednesday of December
}

#[test]
fn test_imm_schedule_start_on_imm_date() {
    // When start is already an IMM date, it should be included
    let start = make_date(2025, 3, 19); // Third Wednesday of March 2025
    let end = make_date(2025, 9, 30);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .imm()
        .build()
        .unwrap()
        .into_iter()
        .collect();

    // Should start from March 19
    assert_eq!(dates.len(), 3);
    assert_eq!(dates[0], make_date(2025, 3, 19)); // March IMM
    assert_eq!(dates[1], make_date(2025, 6, 18)); // June IMM
    assert_eq!(dates[2], make_date(2025, 9, 17)); // September IMM
}

#[test]
fn test_imm_schedule_start_after_first_imm() {
    // Start after March IMM should skip to June
    let start = make_date(2025, 3, 20); // Day after March IMM
    let end = make_date(2025, 9, 30);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .imm()
        .build()
        .unwrap()
        .into_iter()
        .collect();

    // Should start from June IMM
    assert_eq!(dates.len(), 2);
    assert_eq!(dates[0], make_date(2025, 6, 18)); // June IMM
    assert_eq!(dates[1], make_date(2025, 9, 17)); // September IMM
}

#[test]
fn test_imm_schedule_year_rollover() {
    // IMM schedule spanning year boundary
    let start = make_date(2025, 10, 1);
    let end = make_date(2026, 6, 30);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .imm()
        .build()
        .unwrap()
        .into_iter()
        .collect();

    // Dec 2025, Mar 2026, Jun 2026
    assert_eq!(dates.len(), 3);
    assert_eq!(dates[0], make_date(2025, 12, 17)); // December 2025 third Wednesday
    assert_eq!(dates[1], make_date(2026, 3, 18)); // March 2026 third Wednesday
    assert_eq!(dates[2], make_date(2026, 6, 17)); // June 2026 third Wednesday
}

#[test]
fn test_cds_imm_schedule_basic() {
    // CDS IMM schedule: 20th of Mar/Jun/Sep/Dec
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 12, 20);

    let dates: Vec<_> = ScheduleBuilder::new(start, end)
        .cds_imm()
        .build()
        .unwrap()
        .into_iter()
        .collect();

    // Should get 4 dates: Mar 20, Jun 20, Sep 20, Dec 20
    assert_eq!(dates.len(), 4);
    assert_eq!(dates[0], make_date(2025, 3, 20));
    assert_eq!(dates[1], make_date(2025, 6, 20));
    assert_eq!(dates[2], make_date(2025, 9, 20));
    assert_eq!(dates[3], make_date(2025, 12, 20));
}

#[test]
fn test_imm_vs_cds_imm_difference() {
    // Verify that IMM and CDS IMM produce different dates
    // Use end date on a CDS roll date to avoid short back stub
    let start = make_date(2025, 1, 15);
    let end = make_date(2025, 6, 20); // Exactly on CDS roll date

    let imm_dates: Vec<_> = ScheduleBuilder::new(start, end)
        .imm()
        .build()
        .unwrap()
        .into_iter()
        .collect();

    let cds_dates: Vec<_> = ScheduleBuilder::new(start, end)
        .cds_imm()
        .build()
        .unwrap()
        .into_iter()
        .collect();

    // Both should have 2 dates
    assert_eq!(imm_dates.len(), 2);
    assert_eq!(cds_dates.len(), 2);

    // IMM: third Wednesday (Mar 19, Jun 18)
    // CDS: 20th (Mar 20, Jun 20)
    assert_eq!(imm_dates[0], make_date(2025, 3, 19));
    assert_eq!(cds_dates[0], make_date(2025, 3, 20));
    assert_eq!(imm_dates[1], make_date(2025, 6, 18));
    assert_eq!(cds_dates[1], make_date(2025, 6, 20));
}
