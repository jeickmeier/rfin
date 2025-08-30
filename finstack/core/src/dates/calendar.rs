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

#![allow(clippy::assign_op_pattern)]

use time::{Date, Duration, Weekday};

/// Trait representing a holiday calendar.
///
/// Implementors must provide [`HolidayCalendar::is_holiday`].  A blanket
/// [`HolidayCalendar::is_business_day`] implementation is supplied that
/// treats Saturday/Sunday as weekends and defers to `is_holiday` for the
/// remaining non-working days.
pub trait HolidayCalendar {
    /// Returns `true` if `date` is a holiday **or** weekend.
    fn is_holiday(&self, date: Date) -> bool;

    /// Returns `true` if the `date` is a business day according to the
    /// calendar (i.e. *not* weekend and *not* holiday).
    #[inline]
    fn is_business_day(&self, date: Date) -> bool {
        !matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday) && !self.is_holiday(date)
    }
}

/// Common business-day adjustment conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Adjust `date` according to `conv` utilising `cal` for holiday lookup.
pub fn adjust<C: HolidayCalendar + ?Sized>(
    date: Date,
    conv: BusinessDayConvention,
    cal: &C,
) -> Date {
    match conv {
        BusinessDayConvention::Unadjusted => date,
        BusinessDayConvention::Following => {
            if cal.is_business_day(date) {
                return date;
            }
            let mut d = date;
            while !cal.is_business_day(d) {
                d = d + Duration::days(1);
            }
            d
        }
        BusinessDayConvention::ModifiedFollowing => {
            if cal.is_business_day(date) {
                return date;
            }
            let original_month = date.month();

            // Compute following candidate
            let mut forward = date;
            while !cal.is_business_day(forward) {
                forward = forward + Duration::days(1);
            }
            if forward.month() == original_month {
                return forward;
            }

            // Fallback to preceding if following crosses month
            let mut back = date;
            while !cal.is_business_day(back) {
                back = back - Duration::days(1);
            }
            back
        }
        BusinessDayConvention::Preceding => {
            if cal.is_business_day(date) {
                return date;
            }
            let mut d = date;
            while !cal.is_business_day(d) {
                d = d - Duration::days(1);
            }
            d
        }
        BusinessDayConvention::ModifiedPreceding => {
            if cal.is_business_day(date) {
                return date;
            }
            let original_month = date.month();

            // Compute preceding candidate
            let mut back = date;
            while !cal.is_business_day(back) {
                back = back - Duration::days(1);
            }
            if back.month() == original_month {
                return back;
            }

            // Fallback to following if preceding crosses month
            let mut forward = date;
            while !cal.is_business_day(forward) {
                forward = forward + Duration::days(1);
            }
            forward
        }
    }
}

// helper functions merged into `adjust` for a single, cohesive business-day logic surface

// -----------------------------------------------------------------------------
// Runtime discovery helpers
// -----------------------------------------------------------------------------

/// Returns the identifiers of all built-in holiday calendars that have been
/// compiled into the crate.
#[inline]
pub const fn available_calendars() -> &'static [&'static str] {
    crate::dates::holiday::calendars::ALL_IDS
}
