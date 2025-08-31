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

use time::{Date, Duration};
use crate::dates::DateExt;

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
