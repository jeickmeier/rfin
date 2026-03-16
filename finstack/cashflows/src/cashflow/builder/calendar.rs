//! Calendar resolution utilities for cashflow scheduling.
//!
//! Provides a strict resolver that accepts explicit calendar IDs and supports
//! a dedicated weekends-only calendar for cases without holiday rules.

use finstack_core::dates::calendar::Calendar;
use finstack_core::dates::{adjust, BusinessDayConvention, Date};
use finstack_core::dates::{CalendarRegistry, HolidayCalendar};

/// Canonical ID for the weekends-only calendar.
pub const WEEKENDS_ONLY_ID: &str = "weekends_only";

/// Weekends-only calendar for explicit use by callers.
const WEEKENDS_ONLY: Calendar = Calendar::new("weekends_only", "Weekends Only", true, &[]);

/// Resolve a calendar ID to a holiday calendar reference.
///
/// # Errors
///
/// Returns an error if the calendar ID is not recognized.
pub fn resolve_calendar_strict(
    calendar_id: &str,
) -> finstack_core::Result<&'static dyn HolidayCalendar> {
    if calendar_id == WEEKENDS_ONLY_ID {
        return Ok(&WEEKENDS_ONLY);
    }
    CalendarRegistry::global()
        .resolve_str(calendar_id)
        .ok_or_else(|| {
            finstack_core::Error::calendar_not_found_with_suggestions(
                calendar_id,
                CalendarRegistry::global().available_ids(),
            )
        })
}

/// Adjust a single date using the strict calendar policy.
pub fn adjust_date(
    date: Date,
    bdc: BusinessDayConvention,
    calendar_id: &str,
) -> finstack_core::Result<Date> {
    let cal = resolve_calendar_strict(calendar_id)?;
    adjust(date, bdc, cal)
}
