//! Algorithmic holiday helpers centralised for reuse across calendar modules.
//!
//! This module provides shared implementations for Easter Monday and
//! Chinese New Year helpers so they are defined once and reused everywhere.

use time::{Date, Duration, Month};

// -------------------------------------------------------------------------------------------------
// Easter
// -------------------------------------------------------------------------------------------------

/// Compute Easter Monday for a given Gregorian `year`.
#[inline]
pub(crate) fn easter_monday(year: i32) -> Date {
    // Anonymous Gregorian algorithm
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month_num = (h + l - 7 * m + 114) / 31; // 3=March 4=April
    let day = ((h + l - 7 * m + 114) % 31) + 1; // Easter Sunday
    let month = if month_num == 3 {
        Month::March
    } else {
        Month::April
    };
    let easter_sunday = Date::from_calendar_date(year, month, day as u8).unwrap();
    easter_sunday + Duration::days(1) // Easter Monday = Sunday + 1
}

// -------------------------------------------------------------------------------------------------
// Chinese New Year (generated lookup, 1970-2150)
// -------------------------------------------------------------------------------------------------

// The generated table provides `cny_date_for_year` and `is_cny_date` helpers.
include!("../../generated/cny_generated.rs");

/// Return true if `date` is Chinese New Year.
#[inline]
pub(crate) fn is_cny(date: Date) -> bool {
    is_cny_date(date.year(), date.month() as u8, date.day())
}

/// Return the Chinese New Year date for `year`, if available.
#[inline]
pub(crate) fn cny_date(year: i32) -> Option<Date> {
    cny_date_for_year(year)
        .and_then(|(m, d)| Date::from_calendar_date(year, Month::try_from(m).ok()?, d).ok())
}
