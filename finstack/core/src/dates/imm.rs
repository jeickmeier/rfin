//! IMM (International Money Market) / quarterly date helpers.
//!
//! Provides small utility functions for working with standard futures & CDS
//! roll dates used throughout derivative markets:
//!
//! * `third_wednesday(month, year)` – returns the **third Wednesday** of the
//!   given month in the specified Gregorian calendar year.
//! * `next_imm(date)` – returns the **next IMM date** (third Wednesday of
//!   March, June, September or December) *strictly after* the supplied `date`.
//! * `next_cds_date(date)` – returns the **next CDS IMM date** (20-Mar, 20-Jun,
//!   20-Sep, 20-Dec) *strictly after* the supplied `date`.
//!
//! All helpers allocate no heap memory and are
//! panic-free for valid Gregorian dates within the supported `time` range.
//!
//! # Examples
//! ```
//! use finstack_core::dates::{third_wednesday, next_imm, next_cds_date};
//! use time::{Date, Month};
//!
//! let d = third_wednesday(Month::March, 2025);
//! assert_eq!(d, Date::from_calendar_date(2025, Month::March, 19).unwrap());
//!
//! let imm = next_imm(Date::from_calendar_date(2025, Month::March, 20).unwrap());
//! assert_eq!(imm, Date::from_calendar_date(2025, Month::June, 18).unwrap());
//!
//! let cds = next_cds_date(Date::from_calendar_date(2025, Month::March, 10).unwrap());
//! assert_eq!(cds, Date::from_calendar_date(2025, Month::March, 20).unwrap());
//! ```

#![allow(clippy::assign_op_pattern)]

use time::{Date, Duration, Month, Weekday};

/// Return the **third Wednesday** of `month` in `year`.
///
/// The algorithm is a simple deterministic scan starting at the 15th of the
/// month (the earliest possible third Wednesday is the 15th). The loop runs at
/// most seven iterations and is therefore O(1).
///
/// # Panics
/// Never panics for valid Gregorian years supported by the `time` crate.
#[must_use]
pub fn third_wednesday(month: Month, year: i32) -> Date {
    // The third Wednesday is guaranteed to fall within 15..=21 of the month.
    let mut date = Date::from_calendar_date(year, month, 15).unwrap();
    while date.weekday() != Weekday::Wednesday {
        date = date + Duration::days(1);
    }
    date
}

/// Return the **next IMM date** (third Wednesday of Mar/Jun/Sep/Dec) **strictly
/// after** `date`.
#[must_use]
pub fn next_imm(date: Date) -> Date {
    const IMM_MONTHS: [Month; 4] = [Month::March, Month::June, Month::September, Month::December];

    let mut year = date.year();
    loop {
        for &m in &IMM_MONTHS {
            let candidate = third_wednesday(m, year);
            if candidate > date {
                return candidate;
            }
        }
        year += 1; // no candidate in this year ⇒ roll to next year
    }
}

/// Return the **next CDS roll date** (20-Mar/20-Jun/20-Sep/20-Dec) **strictly
/// after** `date`.
#[must_use]
pub fn next_cds_date(date: Date) -> Date {
    const CDS_MONTHS: [Month; 4] = [Month::March, Month::June, Month::September, Month::December];

    let mut year = date.year();
    loop {
        for &m in &CDS_MONTHS {
            // Safe unwrap: 20th exists in every month.
            let candidate = Date::from_calendar_date(year, m, 20).unwrap();
            if candidate > date {
                return candidate;
            }
        }
        year += 1;
    }
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_wed_march_2025() {
        let d = third_wednesday(Month::March, 2025);
        assert_eq!(d, Date::from_calendar_date(2025, Month::March, 19).unwrap());
    }

    #[test]
    fn next_imm_after_mar20_2025() {
        let start = Date::from_calendar_date(2025, Month::March, 20).unwrap();
        let imm = next_imm(start);
        assert_eq!(
            imm,
            Date::from_calendar_date(2025, Month::June, 18).unwrap()
        );
    }

    #[test]
    fn next_cds_before_mar20() {
        let d = Date::from_calendar_date(2025, Month::March, 10).unwrap();
        let cds = next_cds_date(d);
        assert_eq!(
            cds,
            Date::from_calendar_date(2025, Month::March, 20).unwrap()
        );
    }
}
