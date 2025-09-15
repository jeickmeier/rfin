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
//! * `imm_option_expiry(month, year)` – returns the **IMM option expiry date**
//!   (Friday before the third Wednesday) for the given month in the specified year.
//! * `third_friday(month, year)` – returns the **third Friday** of the
//!   given month in the specified Gregorian calendar year.
//! * `next_imm_option_expiry(date)` – returns the **next IMM option expiry date**
//!   *strictly after* the supplied `date`.
//! * `next_equity_option_expiry(date)` – returns the **next equity option expiry date**
//!   (third Friday of any month) *strictly after* the supplied `date`.
//!
//! All helpers allocate no heap memory and are
//! panic-free for valid Gregorian dates within the supported `time` range.
//!
//! # Examples
//! ```
//! use finstack_core::dates::{third_wednesday, next_imm, next_cds_date, imm_option_expiry, third_friday, next_equity_option_expiry};
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
//!
//! let option_expiry = imm_option_expiry(Month::March, 2025);
//! assert_eq!(option_expiry, Date::from_calendar_date(2025, Month::March, 14).unwrap());
//!
//! let equity_expiry = third_friday(Month::March, 2025);
//! assert_eq!(equity_expiry, Date::from_calendar_date(2025, Month::March, 21).unwrap());
//! ```

use time::{Date, Duration, Month, Weekday};

/// Generic helper to find the next date strictly after `date` by scanning
/// specific `months` within a (possibly incrementing) `year`, where candidates
/// are produced by `candidate_fn`.
#[inline]
fn next_date_from_months<F>(date: Date, months: &[Month], candidate_fn: F) -> Date
where
    F: Fn(Month, i32) -> Date,
{
    let mut year = date.year();
    loop {
        for &m in months {
            let candidate = candidate_fn(m, year);
            if candidate > date {
                return candidate;
            }
        }
        year += 1;
    }
}

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
        date += Duration::days(1);
    }
    date
}

/// Return the **next IMM date** (third Wednesday of Mar/Jun/Sep/Dec) **strictly
/// after** `date`.
#[must_use]
pub fn next_imm(date: Date) -> Date {
    const IMM_MONTHS: [Month; 4] = [Month::March, Month::June, Month::September, Month::December];
    next_date_from_months(date, &IMM_MONTHS, third_wednesday)
}

/// Return the **next CDS roll date** (20-Mar/20-Jun/20-Sep/20-Dec) **strictly
/// after** `date`.
#[must_use]
pub fn next_cds_date(date: Date) -> Date {
    const CDS_MONTHS: [Month; 4] = [Month::March, Month::June, Month::September, Month::December];
    next_date_from_months(date, &CDS_MONTHS, |m, year| {
        // Safe unwrap: 20th exists in every month.
        Date::from_calendar_date(year, m, 20).unwrap()
    })
}

/// Return the **IMM option expiry date** (Friday before the third Wednesday) for
/// `month` in `year`.
///
/// IMM option expiry dates typically occur on the Friday preceding the IMM date
/// (third Wednesday). This ensures options expire before the underlying futures
/// contracts for orderly settlement.
///
/// # Panics
/// Never panics for valid Gregorian years supported by the `time` crate.
#[must_use]
pub fn imm_option_expiry(month: Month, year: i32) -> Date {
    let third_wed = third_wednesday(month, year);
    // Friday before Wednesday = subtract 5 days
    third_wed - Duration::days(5)
}

/// Return the **third Friday** of `month` in `year`.
///
/// The algorithm is similar to `third_wednesday`, scanning from the 15th of the
/// month to find the third Friday. The loop runs at most seven iterations and is
/// therefore O(1).
///
/// # Panics
/// Never panics for valid Gregorian years supported by the `time` crate.
#[must_use]
pub fn third_friday(month: Month, year: i32) -> Date {
    // The third Friday is guaranteed to fall within 15..=21 of the month.
    let mut date = Date::from_calendar_date(year, month, 15).unwrap();
    while date.weekday() != Weekday::Friday {
        date += Duration::days(1);
    }
    date
}

/// Return the **next IMM option expiry date** (Friday before third Wednesday of
/// Mar/Jun/Sep/Dec) **strictly after** `date`.
#[must_use]
pub fn next_imm_option_expiry(date: Date) -> Date {
    const IMM_MONTHS: [Month; 4] = [Month::March, Month::June, Month::September, Month::December];
    next_date_from_months(date, &IMM_MONTHS, imm_option_expiry)
}

/// Return the **next equity option expiry date** (third Friday of any month)
/// **strictly after** `date`.
///
/// Equity options typically expire on the third Friday of each month, providing
/// a monthly expiration cycle for equity derivatives.
#[must_use]
pub fn next_equity_option_expiry(date: Date) -> Date {
    let mut year = date.year();
    let mut month = date.month();

    loop {
        let candidate = third_friday(month, year);
        if candidate > date {
            return candidate;
        }

        // Move to next month
        month = month.next();
        if month == Month::January {
            year += 1;
        }
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

    #[test]
    fn imm_option_expiry_march_2025() {
        // March 2025: third Wednesday is 19th, so option expiry is Friday 14th
        let expiry = imm_option_expiry(Month::March, 2025);
        assert_eq!(
            expiry,
            Date::from_calendar_date(2025, Month::March, 14).unwrap()
        );
    }

    #[test]
    fn imm_option_expiry_june_2025() {
        // June 2025: third Wednesday is 18th, so option expiry is Friday 13th
        let expiry = imm_option_expiry(Month::June, 2025);
        assert_eq!(
            expiry,
            Date::from_calendar_date(2025, Month::June, 13).unwrap()
        );
    }

    #[test]
    fn third_friday_march_2025() {
        // March 2025: third Friday is 21st
        let friday = third_friday(Month::March, 2025);
        assert_eq!(
            friday,
            Date::from_calendar_date(2025, Month::March, 21).unwrap()
        );
    }

    #[test]
    fn third_friday_february_2025() {
        // February 2025: third Friday is 21st
        let friday = third_friday(Month::February, 2025);
        assert_eq!(
            friday,
            Date::from_calendar_date(2025, Month::February, 21).unwrap()
        );
    }

    #[test]
    fn next_imm_option_expiry_after_march() {
        // Starting after March 2025 IMM option expiry, should get June 2025
        let start = Date::from_calendar_date(2025, Month::March, 15).unwrap();
        let next_expiry = next_imm_option_expiry(start);
        assert_eq!(
            next_expiry,
            Date::from_calendar_date(2025, Month::June, 13).unwrap()
        );
    }

    #[test]
    fn next_imm_option_expiry_before_march() {
        // Starting before March 2025 IMM option expiry, should get March 2025
        let start = Date::from_calendar_date(2025, Month::March, 10).unwrap();
        let next_expiry = next_imm_option_expiry(start);
        assert_eq!(
            next_expiry,
            Date::from_calendar_date(2025, Month::March, 14).unwrap()
        );
    }

    #[test]
    fn next_equity_option_expiry_mid_march() {
        // Starting mid-March 2025, should get March third Friday (21st)
        let start = Date::from_calendar_date(2025, Month::March, 15).unwrap();
        let next_expiry = next_equity_option_expiry(start);
        assert_eq!(
            next_expiry,
            Date::from_calendar_date(2025, Month::March, 21).unwrap()
        );
    }

    #[test]
    fn next_equity_option_expiry_after_march_friday() {
        // Starting after March third Friday, should get April third Friday
        let start = Date::from_calendar_date(2025, Month::March, 22).unwrap();
        let next_expiry = next_equity_option_expiry(start);
        assert_eq!(
            next_expiry,
            Date::from_calendar_date(2025, Month::April, 18).unwrap()
        );
    }

    #[test]
    fn next_equity_option_expiry_year_rollover() {
        // Starting in December, should roll to January of next year
        let start = Date::from_calendar_date(2025, Month::December, 25).unwrap();
        let next_expiry = next_equity_option_expiry(start);
        assert_eq!(
            next_expiry,
            Date::from_calendar_date(2026, Month::January, 16).unwrap()
        );
    }
}
