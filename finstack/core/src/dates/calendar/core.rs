//! Holiday calendars & business-day adjustment helpers.
//!
//! This module introduces three core building-blocks:
//!
//! 1. [`HolidayCalendar`] – a trait for querying whether a given [`Date`]
//!    is a holiday/business-day for some market.
//! 2. [`BusinessDayConvention`] – an enum of common business-day conventions
//!    (following/preceding, modified, …).
//! 3. [`adjust`] – helper that shifts a date according to a convention
//!    and calendar.
//!
//! In addition, this module includes several built-in holiday calendars
//! (e.g., TARGET2/ECB) that compile unconditionally. They are provided as
//! lightweight, allocation-free implementations.
//!
//! Semantics clarification:
//! - "Holiday" means a non-working date specific to a market calendar. Many
//!   built-in calendars also choose to label weekends as holidays for
//!   convenience, but some calendars intentionally ignore weekends.
//! - Regardless of how a calendar treats weekends in `is_holiday`,
//!   [`HolidayCalendar::is_business_day`] always treats Saturday/Sunday as
//!   non-business days.
//! - Prefer using `is_business_day` for adjustment/scheduling logic. If you
//!   specifically need to check the weekday, use the `is_weekend` helper.

#![allow(clippy::assign_op_pattern)]

use crate::dates::DateExt;
use crate::error::{Error, InputError};
use time::{Date, Duration};

/// Shared upper bound used across business-day search helpers to avoid runaway loops.
pub(crate) const MAX_BUSINESS_DAY_SEARCH_DAYS: i32 = 100;

/// Seek the nearest business day from `date` moving in steps of `step_days` (±1),
/// searching up to `max_days` steps using the provided `cal`.
///
/// Returns an error if no business day is found within the bounded search window.
#[inline]
pub(crate) fn seek_business_day<C: HolidayCalendar + ?Sized>(
    mut date: Date,
    step_days: i32,
    max_days: i32,
    cal: &C,
) -> Result<Date, Error> {
    let mut searched = 0;
    while !cal.is_business_day(date) {
        date = date + Duration::days(step_days as i64);
        searched += 1;
        if searched > max_days {
            return Err(Error::Input(InputError::AdjustmentFailed {
                date: date.to_string(),
                convention: "seek".to_string(),
                max_days,
            }));
        }
    }
    Ok(date)
}

/// Trait representing a holiday calendar.
///
/// Implementors must provide [`HolidayCalendar::is_holiday`]. A blanket
/// [`HolidayCalendar::is_business_day`] implementation is supplied that always
/// treats Saturday/Sunday as weekends (non-business) and defers to
/// `is_holiday` for market-specific non-working days. Some calendars may
/// intentionally return `false` on weekends from `is_holiday` (i.e., they
/// "ignore weekends"); this does not affect `is_business_day` semantics.
pub trait HolidayCalendar {
    /// Returns `true` if `date` is a holiday according to this calendar.
    ///
    /// Notes:
    /// - Many built-in calendars also return `true` for weekends.
    /// - Some calendars intentionally ignore weekends and will return `false`
    ///   for Saturday/Sunday. This only affects `is_holiday`; use
    ///   [`HolidayCalendar::is_business_day`] for business-day logic.
    fn is_holiday(&self, date: Date) -> bool;

    /// Returns `true` if the `date` is a business day according to the
    /// calendar (i.e. not weekend and not holiday).
    ///
    /// Weekend is defined as Saturday/Sunday and is independent of how a
    /// particular calendar chooses to label weekends in [`is_holiday`].
    #[inline]
    fn is_business_day(&self, date: Date) -> bool {
        !date.is_weekend() && !self.is_holiday(date)
    }
}

/// Common business-day adjustment conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BusinessDayConvention {
    /// Leave the date unadjusted (may fall on weekend/holiday).
    Unadjusted,
    /// Next business day (may roll into next month).
    Following,
    /// Following unless that moves the date into the next month – then preceding.
    ModifiedFollowing,
    /// Previous business day (may roll into previous month).
    Preceding,
    /// Preceding unless that moves the date into the previous month – then following.
    ModifiedPreceding,
}

impl core::fmt::Display for BusinessDayConvention {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            BusinessDayConvention::Unadjusted => "Unadjusted",
            BusinessDayConvention::Following => "Following",
            BusinessDayConvention::ModifiedFollowing => "ModifiedFollowing",
            BusinessDayConvention::Preceding => "Preceding",
            BusinessDayConvention::ModifiedPreceding => "ModifiedPreceding",
        };
        f.write_str(s)
    }
}

/// Adjust `date` according to `conv` utilising `cal` for holiday lookup.
///
/// Returns an error if no business day is found within 100 days of the input date.
/// This prevents infinite loops when using composite calendars that mark
/// all days as holidays in a range.
///
/// Example:
/// ```
/// use finstack_core::dates::{Date, BusinessDayConvention, adjust};
/// use finstack_core::dates::calendar::Target2;
/// use time::Month;
/// let cal = Target2;
/// let sat = Date::from_calendar_date(2025, Month::January, 4).unwrap();
/// let adj = adjust(sat, BusinessDayConvention::Following, &cal).unwrap();
/// assert_eq!(adj, Date::from_calendar_date(2025, Month::January, 6).unwrap());
/// ```
pub fn adjust<C: HolidayCalendar + ?Sized>(
    date: Date,
    conv: BusinessDayConvention,
    cal: &C,
) -> Result<Date, Error> {
    match conv {
        BusinessDayConvention::Unadjusted => Ok(date),
        BusinessDayConvention::Following => {
            if cal.is_business_day(date) {
                return Ok(date);
            }
            match seek_business_day(date, 1, MAX_BUSINESS_DAY_SEARCH_DAYS, cal) {
                Ok(d) => Ok(d),
                Err(Error::Input(InputError::AdjustmentFailed { date, max_days, .. })) => {
                    Err(Error::Input(InputError::AdjustmentFailed {
                        date,
                        convention: "Following".to_string(),
                        max_days,
                    }))
                }
                Err(e) => Err(e),
            }
        }
        BusinessDayConvention::ModifiedFollowing => {
            if cal.is_business_day(date) {
                return Ok(date);
            }
            let original_month = date.month();
            let forward = match seek_business_day(date, 1, MAX_BUSINESS_DAY_SEARCH_DAYS, cal) {
                Ok(d) => d,
                Err(Error::Input(InputError::AdjustmentFailed { date, max_days, .. })) => {
                    return Err(Error::Input(InputError::AdjustmentFailed {
                        date,
                        convention: "ModifiedFollowing".to_string(),
                        max_days,
                    }))
                }
                Err(e) => return Err(e),
            };
            if forward.month() == original_month {
                Ok(forward)
            } else {
                match seek_business_day(date, -1, MAX_BUSINESS_DAY_SEARCH_DAYS, cal) {
                    Ok(d) => Ok(d),
                    Err(Error::Input(InputError::AdjustmentFailed { date, max_days, .. })) => {
                        Err(Error::Input(InputError::AdjustmentFailed {
                            date,
                            convention: "ModifiedFollowing".to_string(),
                            max_days,
                        }))
                    }
                    Err(e) => Err(e),
                }
            }
        }
        BusinessDayConvention::Preceding => {
            if cal.is_business_day(date) {
                return Ok(date);
            }
            match seek_business_day(date, -1, MAX_BUSINESS_DAY_SEARCH_DAYS, cal) {
                Ok(d) => Ok(d),
                Err(Error::Input(InputError::AdjustmentFailed { date, max_days, .. })) => {
                    Err(Error::Input(InputError::AdjustmentFailed {
                        date,
                        convention: "Preceding".to_string(),
                        max_days,
                    }))
                }
                Err(e) => Err(e),
            }
        }
        BusinessDayConvention::ModifiedPreceding => {
            if cal.is_business_day(date) {
                return Ok(date);
            }
            let original_month = date.month();
            let back = match seek_business_day(date, -1, MAX_BUSINESS_DAY_SEARCH_DAYS, cal) {
                Ok(d) => d,
                Err(Error::Input(InputError::AdjustmentFailed { date, max_days, .. })) => {
                    return Err(Error::Input(InputError::AdjustmentFailed {
                        date,
                        convention: "ModifiedPreceding".to_string(),
                        max_days,
                    }))
                }
                Err(e) => return Err(e),
            };
            if back.month() == original_month {
                Ok(back)
            } else {
                match seek_business_day(date, 1, MAX_BUSINESS_DAY_SEARCH_DAYS, cal) {
                    Ok(d) => Ok(d),
                    Err(Error::Input(InputError::AdjustmentFailed { date, max_days, .. })) => {
                        Err(Error::Input(InputError::AdjustmentFailed {
                            date,
                            convention: "ModifiedPreceding".to_string(),
                            max_days,
                        }))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Runtime discovery helpers
// -----------------------------------------------------------------------------

/// Returns the identifiers of all built-in holiday calendars that have been
/// compiled into the crate.
///
/// Identifiers are lowercase, stable market codes (e.g., `"gblo"`, `"target2"`).
/// They are suitable for serialization and long-lived pipelines.
///
/// Example using the registry:
/// ```
/// use finstack_core::dates::calendar::registry::{CalendarId, CalendarRegistry};
/// let regs = CalendarRegistry::global();
/// let ids = regs.available_ids();
/// let maybe = regs.resolve(CalendarId(ids[0]));
/// assert!(maybe.is_some());
/// ```
#[inline]
pub const fn available_calendars() -> &'static [&'static str] {
    crate::dates::calendar::ALL_IDS
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn test_business_day_convention_serde_roundtrip() {
        use serde_json;

        // Test all BusinessDayConvention variants
        let conventions = vec![
            BusinessDayConvention::Unadjusted,
            BusinessDayConvention::Following,
            BusinessDayConvention::ModifiedFollowing,
            BusinessDayConvention::Preceding,
            BusinessDayConvention::ModifiedPreceding,
        ];

        for conv in conventions {
            let json = serde_json::to_string(&conv).unwrap();
            let deserialized: BusinessDayConvention = serde_json::from_str(&json).unwrap();
            assert_eq!(conv, deserialized);
        }
    }

    #[test]
    fn test_business_day_convention_snake_case() {
        use serde_json;

        // Test that the snake_case renaming works
        let conv = BusinessDayConvention::ModifiedFollowing;
        let json = serde_json::to_string(&conv).unwrap();
        assert_eq!(json, "\"modified_following\"");

        let conv = BusinessDayConvention::ModifiedPreceding;
        let json = serde_json::to_string(&conv).unwrap();
        assert_eq!(json, "\"modified_preceding\"");
    }
}
