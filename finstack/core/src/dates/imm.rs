//! IMM (International Money Market) and quarterly roll date helpers.
//!
//! Provides deterministic, allocation-free utilities for computing standard
//! futures roll dates, CDS settlement dates, and option expiry dates used
//! throughout global derivative markets.
//!
//! # Features
//!
//! - **IMM dates**: Third Wednesday of March, June, September, December
//! - **CDS IMM dates**: 20th of March, June, September, December
//! - **Option expiry**: Friday before IMM date (futures options)
//! - **Equity expiry**: Third Friday of every month
//! - **Zero allocation**: All functions are stack-only
//! - **Panic-free**: Safe for all valid `time::Date` ranges
//!
//! # Background
//!
//! The International Money Market (IMM) dates are standardized quarterly
//! roll dates used for futures contracts (interest rate futures, currency
//! futures, equity index futures) and credit default swaps. These dates
//! ensure coordinated settlement across global derivatives markets.
//!
//! ## IMM Dates (Third Wednesday)
//!
//! - Used by: CME futures, CBOE index options, many OTC derivatives
//! - Convention: Third Wednesday of March, June, September, December
//! - Example: March 19, 2025 is an IMM date
//!
//! ## CDS IMM Dates (20th of Quarter Month)
//!
//! - Used by: Credit default swaps, credit indices (CDX, iTraxx)
//! - Convention: 20th of March, June, September, December
//! - Rationale: Standardized by ISDA Big Bang Protocol (2009)
//!
//! ## Option Expiry Dates
//!
//! - **IMM option expiry**: Friday before third Wednesday (futures options)
//! - **Equity option expiry**: Third Friday of every month (equity derivatives)
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_core::dates::{third_wednesday, next_imm, next_cds_date};
//! use time::{Date, Month};
//!
//! // IMM date for a specific month
//! let imm_march = third_wednesday(Month::March, 2025);
//! assert_eq!(imm_march, Date::from_calendar_date(2025, Month::March, 19).unwrap());
//!
//! // Find next IMM date after a given date
//! let date = Date::from_calendar_date(2025, Month::March, 20).unwrap();
//! let next = next_imm(date);
//! assert_eq!(next, Date::from_calendar_date(2025, Month::June, 18).unwrap());
//!
//! // CDS settlement date
//! let cds = next_cds_date(Date::from_calendar_date(2025, Month::March, 10).unwrap());
//! assert_eq!(cds, Date::from_calendar_date(2025, Month::March, 20).unwrap());
//! ```
//!
//! # Standards Reference
//!
//! - **IMM dates**: CME Group contract specifications
//! - **CDS dates**: ISDA Big Bang Protocol (April 2009), ISDA CDS Standard Model
//! - **Equity options**: CBOE, major equity exchanges worldwide
//!
//! # See Also
//!
//! - [`ScheduleBuilder::cds_imm`] for building CDS payment schedules
//! - [`next_imm`] for finding the next quarterly IMM date
//! - [`next_cds_date`] for CDS settlement date calculation
//!
//! [`ScheduleBuilder::cds_imm`]: super::ScheduleBuilder::cds_imm

use crate::dates::calendar::generated::nth_weekday_of_month;
use time::{Date, Duration, Month, Weekday};

// Shared quarter months used by IMM/CDS roll helpers
const QUARTERLY_MONTHS: [Month; 4] = [Month::March, Month::June, Month::September, Month::December];

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
/// Delegates to the shared `nth_weekday_of_month` helper used by calendar rules
/// to keep all "nth weekday" logic consistent.
///
/// # Panics
/// Never panics for valid Gregorian years supported by the `time` crate.
#[must_use]
pub fn third_wednesday(month: Month, year: i32) -> Date {
    nth_weekday_of_month(year, month, Weekday::Wednesday, 3)
}

/// Return the **next IMM date** (third Wednesday of Mar/Jun/Sep/Dec) **strictly
/// after** `date`.
#[must_use]
pub fn next_imm(date: Date) -> Date {
    next_date_from_months(date, &QUARTERLY_MONTHS, third_wednesday)
}

/// Return the **next CDS roll date** (20-Mar/20-Jun/20-Sep/20-Dec) **strictly
/// after** `date`.
#[must_use]
pub fn next_cds_date(date: Date) -> Date {
    next_date_from_months(date, &QUARTERLY_MONTHS, |m, year| {
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
/// Delegates to the shared `nth_weekday_of_month` helper used by calendar rules
/// to keep all "nth weekday" logic consistent.
///
/// # Panics
/// Never panics for valid Gregorian years supported by the `time` crate.
#[must_use]
pub fn third_friday(month: Month, year: i32) -> Date {
    nth_weekday_of_month(year, month, Weekday::Friday, 3)
}

/// Return the **next IMM option expiry date** (Friday before third Wednesday of
/// Mar/Jun/Sep/Dec) **strictly after** `date`.
#[must_use]
pub fn next_imm_option_expiry(date: Date) -> Date {
    next_date_from_months(date, &QUARTERLY_MONTHS, imm_option_expiry)
}

/// Return the **next equity option expiry date** (third Friday of any month)
/// **strictly after** `date`.
///
/// Equity options typically expire on the third Friday of each month, providing
/// a monthly expiration cycle for equity derivatives.
#[must_use]
pub fn next_equity_option_expiry(date: Date) -> Date {
    const ALL_MONTHS: [Month; 12] = [
        Month::January,
        Month::February,
        Month::March,
        Month::April,
        Month::May,
        Month::June,
        Month::July,
        Month::August,
        Month::September,
        Month::October,
        Month::November,
        Month::December,
    ];

    next_date_from_months(date, &ALL_MONTHS, third_friday)
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
