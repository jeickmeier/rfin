//! Additional coverage for business day adjustment edge cases.

use super::common::{make_date, TestCal};
use finstack_core::dates::calendar::business_days::{BusinessDayConvention, CalendarMetadata, HolidayCalendar};

/// Custom calendar that exposes metadata for testing.
struct MetadataCal;

impl HolidayCalendar for MetadataCal {
    fn is_holiday(&self, date: finstack_core::dates::Date) -> bool {
        // Treat only January 1st as a holiday to keep behaviour simple.
        date.month() == time::Month::January && date.day() == 1
    }

    fn metadata(&self) -> Option<CalendarMetadata> {
        Some(CalendarMetadata {
            id: "meta-cal",
            name: "Metadata Calendar",
            ignore_weekends: false,
        })
    }
}

#[test]
fn following_skips_multiple_consecutive_holidays() {
    // Set up calendar with back-to-back weekday holidays surrounding a weekend.
    let cal = TestCal::new()
        .with_holiday(make_date(2025, 7, 3))  // Thursday
        .with_holiday(make_date(2025, 7, 4)); // Friday (Independence Day)

    // Start from Thursday holiday: should roll forward to Monday July 7th.
    let thursday = make_date(2025, 7, 3);
    let adjusted = finstack_core::dates::adjust(
        thursday,
        BusinessDayConvention::Following,
        &cal,
    )
    .expect("Following adjustment should succeed");
    assert_eq!(adjusted, make_date(2025, 7, 7));
}

#[test]
fn preceding_skips_multiple_consecutive_holidays() {
    let cal = TestCal::new()
        .with_holiday(make_date(2025, 12, 29)) // Monday
        .with_holiday(make_date(2025, 12, 30)); // Tuesday

    let holiday = make_date(2025, 12, 30);
    let adjusted = finstack_core::dates::adjust(
        holiday,
        BusinessDayConvention::Preceding,
        &cal,
    )
    .expect("Preceding adjustment should succeed");
    assert_eq!(adjusted, make_date(2025, 12, 26)); // Friday, skipping weekend + holidays
}

#[test]
fn business_day_convention_display_strings() {
    let values = [
        (BusinessDayConvention::Unadjusted, "Unadjusted"),
        (BusinessDayConvention::Following, "Following"),
        (BusinessDayConvention::ModifiedFollowing, "ModifiedFollowing"),
        (BusinessDayConvention::Preceding, "Preceding"),
        (
            BusinessDayConvention::ModifiedPreceding,
            "ModifiedPreceding",
        ),
    ];

    for (conv, expected) in values {
        assert_eq!(format!("{}", conv), expected);
    }
}

#[test]
fn calendar_metadata_override_is_respected() {
    let cal = MetadataCal;

    let metadata = cal.metadata().expect("metadata should be present");
    assert_eq!(metadata.id, "meta-cal");
    assert_eq!(metadata.name, "Metadata Calendar");
    assert!(!metadata.ignore_weekends);

    // Non-holiday weekday should be business day.
    let jan2 = make_date(2025, 1, 2);
    assert!(cal.is_business_day(jan2));

    // Holiday should not be business day.
    let new_year = make_date(2025, 1, 1);
    assert!(!cal.is_business_day(new_year));
}
