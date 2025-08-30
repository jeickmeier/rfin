//! Date utilities shared across the `dates` module.

use time::{Date, Month};

/// Return true if `year` is a leap year in the Gregorian calendar.
#[inline]
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Add `months` to `date`, preserving the day when possible.
///
/// Handles negative month offsets correctly and clamps the day to the last
/// valid day for the target month (e.g. Jan 31 + 1 month → Feb 28/29).
#[inline]
pub fn add_months(date: Date, months: i32) -> Date {
    let total_months = date.year() * 12 + (date.month() as i32 - 1) + months;
    let new_year = total_months.div_euclid(12);
    let new_month_index = total_months.rem_euclid(12);
    let new_month = Month::try_from((new_month_index + 1) as u8).unwrap();

    let day = date.day();
    let max_day = match new_month {
        Month::January => 31,
        Month::February => {
            if is_leap_year(new_year) { 29 } else { 28 }
        }
        Month::March => 31,
        Month::April => 30,
        Month::May => 31,
        Month::June => 30,
        Month::July => 31,
        Month::August => 31,
        Month::September => 30,
        Month::October => 31,
        Month::November => 30,
        Month::December => 31,
    };

    let new_day = day.min(max_day);
    Date::from_calendar_date(new_year, new_month, new_day).unwrap()
}

/// Convert a `Date` to the number of days since the Unix epoch (1970-01-01).
#[inline]
pub fn date_to_days_since_epoch(date: Date) -> i32 {
    let epoch = Date::from_calendar_date(1970, Month::January, 1).unwrap();
    (date - epoch).whole_days() as i32
}

/// Convert days since Unix epoch (1970-01-01) back to a `Date`.
#[inline]
pub fn days_since_epoch_to_date(days: i32) -> Date {
    let epoch = Date::from_calendar_date(1970, Month::January, 1).unwrap();
    epoch + time::Duration::days(days as i64)
}


