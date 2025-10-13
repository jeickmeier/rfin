//! Tests for calendar adjustment functionality.

use super::common::{make_date, TestCal};
use finstack_core::dates::{adjust, available_calendars, BusinessDayConvention, HolidayCalendar};
use finstack_core::error::InputError;
use time::Date;

#[test]
fn test_holiday_calendar_trait() {
    let cal = TestCal::new().with_holiday(make_date(2025, 1, 1)); // New Year's Day (Wednesday)

    // Wednesday New Year should be a holiday
    assert!(cal.is_holiday(make_date(2025, 1, 1)));

    // Regular weekday should not be a holiday
    assert!(!cal.is_holiday(make_date(2025, 1, 2))); // Thursday

    // Weekend should not be a business day (handled by trait default)
    assert!(!cal.is_business_day(make_date(2025, 1, 4))); // Saturday
    assert!(!cal.is_business_day(make_date(2025, 1, 5))); // Sunday

    // Holiday should not be a business day
    assert!(!cal.is_business_day(make_date(2025, 1, 1))); // New Year

    // Regular weekday should be a business day
    assert!(cal.is_business_day(make_date(2025, 1, 2))); // Thursday
}

#[test]
fn test_adjust_unadjusted() {
    let cal = TestCal::new();
    let saturday = make_date(2025, 1, 4); // Saturday

    // Unadjusted should return the same date even if it's a weekend
    let result = adjust(saturday, BusinessDayConvention::Unadjusted, &cal).unwrap();
    assert_eq!(result, saturday);
}

#[test]
fn test_adjust_following() {
    let cal = TestCal::new().with_holiday(make_date(2025, 1, 2)); // Thursday holiday

    // Saturday should move to Monday
    let saturday = make_date(2025, 1, 4);
    let result = adjust(saturday, BusinessDayConvention::Following, &cal).unwrap();
    assert_eq!(result, make_date(2025, 1, 6)); // Monday

    // Holiday Thursday should move to Friday
    let thursday = make_date(2025, 1, 2);
    let result = adjust(thursday, BusinessDayConvention::Following, &cal).unwrap();
    assert_eq!(result, make_date(2025, 1, 3)); // Friday

    // Business day should remain unchanged
    let friday = make_date(2025, 1, 3);
    let result = adjust(friday, BusinessDayConvention::Following, &cal).unwrap();
    assert_eq!(result, friday);
}

#[test]
fn test_adjust_preceding() {
    let cal = TestCal::new().with_holiday(make_date(2025, 1, 3)); // Friday holiday

    // Sunday should move to Friday (but Friday is holiday, so Thursday)
    let sunday = make_date(2025, 1, 5);
    let result = adjust(sunday, BusinessDayConvention::Preceding, &cal).unwrap();
    assert_eq!(result, make_date(2025, 1, 2)); // Thursday

    // Holiday Friday should move to Thursday
    let friday = make_date(2025, 1, 3);
    let result = adjust(friday, BusinessDayConvention::Preceding, &cal).unwrap();
    assert_eq!(result, make_date(2025, 1, 2)); // Thursday
}

#[test]
fn test_adjust_modified_following() {
    let cal = TestCal::new();

    // End of month Saturday that would roll to next month
    let jan25_sat = make_date(2025, 1, 25); // Saturday

    // This should go following (Monday) since it stays in same month
    let result = adjust(jan25_sat, BusinessDayConvention::ModifiedFollowing, &cal).unwrap();
    assert_eq!(result, make_date(2025, 1, 27)); // Monday

    // Test a case where following would cross month boundary
    // January 31, 2025 is a Friday, so let's use January 30 (Thursday) as a holiday
    let cal_with_holiday = TestCal::new()
        .with_holiday(make_date(2025, 1, 30)) // Thursday
        .with_holiday(make_date(2025, 1, 31)); // Friday

    let jan30 = make_date(2025, 1, 30);
    let result = adjust(
        jan30,
        BusinessDayConvention::ModifiedFollowing,
        &cal_with_holiday,
    )
    .unwrap();
    // Following would go to Feb 3 (Monday), but that crosses month, so go preceding to Jan 29
    assert_eq!(result, make_date(2025, 1, 29)); // Wednesday
}

#[test]
fn test_adjust_modified_preceding() {
    let cal = TestCal::new().with_holiday(make_date(2025, 2, 3)); // Monday holiday

    // February 1, 2025 is a Saturday
    let feb1_sat = make_date(2025, 2, 1);
    let result = adjust(feb1_sat, BusinessDayConvention::ModifiedPreceding, &cal).unwrap();
    // Preceding would go to Jan 31, but that crosses month, so go following to Feb 4
    assert_eq!(result, make_date(2025, 2, 4)); // Tuesday (since Feb 3 is holiday)
}

#[test]
fn test_available_calendars() {
    let calendars = available_calendars();

    // Should have some calendars
    assert!(!calendars.is_empty());

    // Should contain some expected calendars
    assert!(calendars.contains(&"gblo"));
    assert!(calendars.contains(&"target2"));
}

/// Test calendar that marks ALL days as holidays to trigger infinite loop scenarios.
struct AllHolidaysCal;

impl HolidayCalendar for AllHolidaysCal {
    fn is_holiday(&self, _date: Date) -> bool {
        true // Every day is a holiday
    }
}

#[test]
fn test_adjust_following_infinite_loop_guard() {
    let cal = AllHolidaysCal;
    let date = make_date(2025, 1, 1);

    // This should return an error after 100 days instead of looping forever
    let result = adjust(date, BusinessDayConvention::Following, &cal);
    assert!(result.is_err());

    // Verify the error type and message
    match result.unwrap_err() {
        finstack_core::Error::Input(InputError::AdjustmentFailed {
            date: _,
            convention,
            max_days,
        }) => {
            assert_eq!(convention, BusinessDayConvention::Following);
            assert_eq!(max_days, 100);
        }
        other => panic!("Expected AdjustmentFailed error, got {:?}", other),
    }
}

#[test]
fn test_adjust_preceding_infinite_loop_guard() {
    let cal = AllHolidaysCal;
    let date = make_date(2025, 1, 1);

    // This should return an error after 100 days instead of looping forever
    let result = adjust(date, BusinessDayConvention::Preceding, &cal);
    assert!(result.is_err());

    // Verify the error type and message
    match result.unwrap_err() {
        finstack_core::Error::Input(InputError::AdjustmentFailed {
            date: _,
            convention,
            max_days,
        }) => {
            assert_eq!(convention, BusinessDayConvention::Preceding);
            assert_eq!(max_days, 100);
        }
        other => panic!("Expected AdjustmentFailed error, got {:?}", other),
    }
}

#[test]
fn test_adjust_modified_following_infinite_loop_guard() {
    let cal = AllHolidaysCal;
    let date = make_date(2025, 1, 1);

    // This should return an error when trying to find a following business day
    let result = adjust(date, BusinessDayConvention::ModifiedFollowing, &cal);
    assert!(result.is_err());

    // Verify the error type and message
    match result.unwrap_err() {
        finstack_core::Error::Input(InputError::AdjustmentFailed {
            date: _,
            convention,
            max_days,
        }) => {
            assert_eq!(convention, BusinessDayConvention::ModifiedFollowing);
            assert_eq!(max_days, 100);
        }
        other => panic!("Expected AdjustmentFailed error, got {:?}", other),
    }
}

#[test]
fn test_adjust_modified_preceding_infinite_loop_guard() {
    let cal = AllHolidaysCal;
    let date = make_date(2025, 1, 1);

    // This should return an error when trying to find a preceding business day
    let result = adjust(date, BusinessDayConvention::ModifiedPreceding, &cal);
    assert!(result.is_err());

    // Verify the error type and message
    match result.unwrap_err() {
        finstack_core::Error::Input(InputError::AdjustmentFailed {
            date: _,
            convention,
            max_days,
        }) => {
            assert_eq!(convention, BusinessDayConvention::ModifiedPreceding);
            assert_eq!(max_days, 100);
        }
        other => panic!("Expected AdjustmentFailed error, got {:?}", other),
    }
}

#[test]
fn test_improved_error_messages_contain_original_date_and_correct_convention() {
    let cal = AllHolidaysCal;
    let original_date = make_date(2025, 1, 1);

    // Test Following convention error contains correct information
    let result = adjust(original_date, BusinessDayConvention::Following, &cal);
    assert!(result.is_err());

    match result.unwrap_err() {
        finstack_core::Error::Input(InputError::AdjustmentFailed {
            date,
            convention,
            max_days,
        }) => {
            // Verify the error contains the ORIGINAL date (not the last attempted date)
            assert_eq!(date, original_date);
            // Verify the error contains the correct convention enum
            assert_eq!(convention, BusinessDayConvention::Following);
            assert_eq!(max_days, 100);
        }
        other => panic!("Expected AdjustmentFailed error, got {:?}", other),
    }

    // Test Preceding convention error contains correct information
    let result = adjust(original_date, BusinessDayConvention::Preceding, &cal);
    assert!(result.is_err());

    match result.unwrap_err() {
        finstack_core::Error::Input(InputError::AdjustmentFailed {
            date,
            convention,
            max_days,
        }) => {
            // Verify the error contains the ORIGINAL date (not the last attempted date)
            assert_eq!(date, original_date);
            // Verify the error contains the correct convention enum
            assert_eq!(convention, BusinessDayConvention::Preceding);
            assert_eq!(max_days, 100);
        }
        other => panic!("Expected AdjustmentFailed error, got {:?}", other),
    }

    // Test ModifiedFollowing convention error contains correct information
    let result = adjust(
        original_date,
        BusinessDayConvention::ModifiedFollowing,
        &cal,
    );
    assert!(result.is_err());

    match result.unwrap_err() {
        finstack_core::Error::Input(InputError::AdjustmentFailed {
            date,
            convention,
            max_days,
        }) => {
            // Verify the error contains the ORIGINAL date (not the last attempted date)
            assert_eq!(date, original_date);
            // Verify the error contains the correct convention enum
            assert_eq!(convention, BusinessDayConvention::ModifiedFollowing);
            assert_eq!(max_days, 100);
        }
        other => panic!("Expected AdjustmentFailed error, got {:?}", other),
    }

    // Test ModifiedPreceding convention error contains correct information
    let result = adjust(
        original_date,
        BusinessDayConvention::ModifiedPreceding,
        &cal,
    );
    assert!(result.is_err());

    match result.unwrap_err() {
        finstack_core::Error::Input(InputError::AdjustmentFailed {
            date,
            convention,
            max_days,
        }) => {
            // Verify the error contains the ORIGINAL date (not the last attempted date)
            assert_eq!(date, original_date);
            // Verify the error contains the correct convention enum
            assert_eq!(convention, BusinessDayConvention::ModifiedPreceding);
            assert_eq!(max_days, 100);
        }
        other => panic!("Expected AdjustmentFailed error, got {:?}", other),
    }
}
