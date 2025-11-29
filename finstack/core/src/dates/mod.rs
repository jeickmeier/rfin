//! Date utilities: business-day adjustments, day-counts, schedules, IMM helpers.
//!
//! This module wraps the [`time`](https://docs.rs/time) crate and exposes
//! domain-specific helpers commonly needed by pricing engines. Everything is
//! available through [`finstack_core::dates`], keeping downstream dependencies
//! small and version-stable.
//!
//! # Highlights
//! - ergonomic re-exports of `time` primitives (`Date`, `OffsetDateTime`, …)
//! - holiday calendars and business-day conventions (`adjust`, `HolidayCalendar`)
//! - schedule generation utilities (`ScheduleBuilder`)
//! - IMM/third-Wednesday helpers for derivatives roll dates
//!
//! # Examples
//! ```rust
//! use finstack_core::dates::{
//!     adjust, build_periods, BusinessDayConvention, Date, Frequency, ScheduleBuilder, create_date,
//! };
//! use finstack_core::dates::calendar::TARGET2;
//! use time::{Duration, Month};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//! let trade_date = create_date(2024, Month::March, 29)?;
//! let adjusted = adjust(trade_date, BusinessDayConvention::Following, &TARGET2)?;
//! assert!(adjusted >= trade_date);
//!
//! let end = trade_date + Duration::days(365);
//! let schedule = ScheduleBuilder::new(trade_date, end)
//!     .frequency(Frequency::quarterly())
//!     .build()?;
//! assert!(schedule.dates.len() >= 4);
//!
//! let periods = build_periods("2024Q1..Q4", None)?;
//! assert_eq!(periods.periods.len(), 4);
//! # Ok(())
//! # }
//! ```

// ----------------------------------------------------------------------------------
// Re-exports – keep list short & focused
// ----------------------------------------------------------------------------------

pub use time::{Date, OffsetDateTime, PrimitiveDateTime};

// Build-time bitsets removed the last runtime use of DateBuf; keep only if needed elsewhere.

mod date_extensions;

// Publicly re-export the extension traits so downstream crates can `use finstack_core::dates::DateExt`.
pub use date_extensions::{DateExt, OffsetDateTimeExt};

mod daycount;

pub use daycount::{DayCount, DayCountCtx, DayCountCtxState, Thirty360Convention};

pub mod rate_conversions;

// Re-export new holiday calendars at the top level for convenience
pub use calendar::business_days::{adjust, BusinessDayConvention, HolidayCalendar};

// The canonical public discovery helper
pub use calendar::business_days::available_calendars;

mod schedule_iter;

pub use schedule_iter::{Frequency, Schedule, ScheduleBuilder, ScheduleSpec, StubKind};

pub use calendar::composite::CompositeCalendar;

mod imm;
mod tenor;

pub use tenor::{Tenor, TenorUnit};

pub use imm::{
    imm_option_expiry, next_cds_date, next_equity_option_expiry, next_imm, next_imm_option_expiry,
    third_friday, third_wednesday,
};

pub mod calendar;
pub use calendar::registry::CalendarRegistry;

mod periods;
pub use periods::{
    build_fiscal_periods, build_periods, FiscalConfig, Period, PeriodId, PeriodKey, PeriodKind,
};

/// Safe date creation helper that returns a Result instead of panicking.
///
/// This is a safer alternative to `Date::from_calendar_date(...).unwrap()`
/// that provides proper error handling for invalid dates like February 30th.
///
/// # Examples
/// ```rust
/// use finstack_core::dates::create_date;
/// use time::Month;
///
/// // Valid date
/// let date = create_date(2025, Month::January, 15)?;
///
/// // Invalid date - returns error instead of panic
/// let result = create_date(2025, Month::February, 30); // Returns Err
/// # Ok::<(), finstack_core::Error>(())
/// ```
pub fn create_date(year: i32, month: time::Month, day: u8) -> crate::Result<Date> {
    Date::from_calendar_date(year, month, day)
        .map_err(|_| crate::error::InputError::InvalidDate {
            year,
            month: month as u8,
            day,
        })
        .map_err(Into::into)
}

/// Add months to a date, clamping to the last valid day of the target month.
///
/// This is a convenience function that wraps [`DateExt::add_months`].
/// For more ergonomic usage, consider importing [`DateExt`] and using the method directly.
///
/// # Example
/// ```rust
/// use finstack_core::dates::add_months;
/// use time::{Date, Month};
///
/// let date = Date::from_calendar_date(2024, Month::January, 31).expect("Valid date");
/// assert_eq!(add_months(date, 1), Date::from_calendar_date(2024, Month::February, 29).expect("Valid date"));
/// ```
pub fn add_months(date: Date, months: i32) -> Date {
    date.add_months(months)
}

/// Calculate the number of whole months between two dates.
///
/// This is a convenience function that wraps [`DateExt::months_until`].
/// Returns `(to.year - from.year) * 12 + (to.month - from.month)`.
/// If `to` is before `from`, returns `0`.
///
/// This is commonly used to calculate loan seasoning (age) in months for
/// structured credit instruments.
///
/// # Example
/// ```rust
/// use finstack_core::dates::months_between;
/// use time::{Date, Month};
///
/// let from = Date::from_calendar_date(2020, Month::January, 15).expect("Valid date");
/// let to = Date::from_calendar_date(2022, Month::March, 10).expect("Valid date");
/// assert_eq!(months_between(from, to), 26);
/// ```
pub fn months_between(from: Date, to: Date) -> u32 {
    from.months_until(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_create_date_valid_dates() {
        // Test normal valid dates
        assert!(create_date(2025, Month::January, 15).is_ok());
        assert!(create_date(2024, Month::February, 29).is_ok()); // Leap year
        assert!(create_date(2000, Month::February, 29).is_ok()); // Leap year (century divisible by 400)
        assert!(create_date(2023, Month::December, 31).is_ok());
        assert!(create_date(1970, Month::January, 1).is_ok()); // Unix epoch
    }

    #[test]
    fn test_create_date_invalid_dates() {
        // Test invalid February dates
        let result = create_date(2023, Month::February, 29);
        assert!(result.is_err());
        match result.expect_err("Should fail for invalid date") {
            crate::Error::Input(crate::error::InputError::InvalidDate { year, month, day }) => {
                assert_eq!(year, 2023);
                assert_eq!(month, 2); // February
                assert_eq!(day, 29);
            }
            _ => panic!("Expected InvalidDate error"),
        }

        // Test non-leap year February 29
        let result = create_date(2023, Month::February, 29);
        assert!(result.is_err());

        // Test invalid day numbers
        assert!(create_date(2023, Month::January, 0).is_err());
        assert!(create_date(2023, Month::January, 32).is_err());
        assert!(create_date(2023, Month::April, 31).is_err()); // April has 30 days
        assert!(create_date(2023, Month::June, 31).is_err()); // June has 30 days

        // Test extreme invalid dates
        assert!(create_date(2023, Month::January, 255).is_err());
    }

    #[test]
    fn test_create_date_error_message_format() {
        let result = create_date(2023, Month::February, 30);
        assert!(result.is_err());

        let error_msg = format!("{}", result.expect_err("Should fail for invalid date"));
        assert!(error_msg.contains("Invalid calendar date"));
        assert!(error_msg.contains("2023-02-30"));
    }

    #[test]
    fn test_create_date_edge_cases() {
        // Test month boundaries
        assert!(create_date(2023, Month::January, 31).is_ok());
        assert!(create_date(2023, Month::March, 31).is_ok());
        assert!(create_date(2023, Month::April, 30).is_ok());
        assert!(create_date(2023, Month::May, 31).is_ok());

        // Test leap year edge cases
        assert!(create_date(1900, Month::February, 28).is_ok()); // Not leap year (century not divisible by 400)
        assert!(create_date(1900, Month::February, 29).is_err()); // Not leap year
        assert!(create_date(2000, Month::February, 29).is_ok()); // Leap year (century divisible by 400)
        assert!(create_date(2100, Month::February, 28).is_ok()); // Not leap year
        assert!(create_date(2100, Month::February, 29).is_err()); // Not leap year
    }

    #[test]
    fn test_create_date_year_boundaries() {
        // Test reasonable year boundaries
        assert!(create_date(-9999, Month::January, 1).is_ok());
        assert!(create_date(9999, Month::December, 31).is_ok());

        // Test year 0 (should be valid in time crate)
        assert!(create_date(0, Month::January, 1).is_ok());
    }

    #[test]
    fn test_create_date_comparison_with_direct_usage() {
        // Verify that create_date produces the same results as direct Date::from_calendar_date for valid dates
        let valid_cases = vec![
            (2023, Month::January, 15),
            (2024, Month::February, 29), // Leap year
            (2023, Month::December, 31),
            (1970, Month::January, 1),
        ];

        for (year, month, day) in valid_cases {
            let direct_result = Date::from_calendar_date(year, month, day);
            let helper_result = create_date(year, month, day);

            assert!(direct_result.is_ok());
            assert!(helper_result.is_ok());
            assert_eq!(
                direct_result.expect("Direct result should succeed in test"),
                helper_result.expect("Helper result should succeed in test")
            );
        }
    }
}
