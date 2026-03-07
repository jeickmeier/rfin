//! Payment delay conventions for agency MBS.
//!
//! Agency MBS have standardized delays between the end of an accrual period
//! and the actual payment date:
//!
//! - **FNMA**: 55 calendar days (actual stated delay)
//! - **FHLMC**: 75 calendar days (actual stated delay)
//! - **GNMA I**: 14 calendar days (single-issuer pools, payment on the 15th)
//! - **GNMA II**: 45 calendar days (multi-issuer pools, payment on the 20th)
//!
//! The delay is measured from the accrual period end (typically the last day
//! of the month) to the payment date.

use crate::instruments::fixed_income::mbs_passthrough::AgencyProgram;
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{BusinessDayConvention, Date};
use finstack_core::Result;

/// Get the standard payment delay in days for an agency program.
///
/// # Arguments
///
/// * `agency` - Agency program (FNMA, FHLMC, GNMA)
///
/// # Returns
///
/// Payment delay in calendar days
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::mbs_passthrough::{
///     AgencyProgram,
///     delay::payment_lag_days,
/// };
///
/// assert_eq!(payment_lag_days(AgencyProgram::Fnma), 55);
/// assert_eq!(payment_lag_days(AgencyProgram::Fhlmc), 55);
/// assert_eq!(payment_lag_days(AgencyProgram::GnmaI), 14);
/// assert_eq!(payment_lag_days(AgencyProgram::GnmaII), 50);
/// ```
pub fn payment_lag_days(agency: AgencyProgram) -> u32 {
    agency.payment_lag_days()
}

/// Calculate actual payment date from accrual period end.
///
/// Adds the payment delay and optionally adjusts for business days.
///
/// # Arguments
///
/// * `accrual_end` - End date of the accrual period
/// * `delay_days` - Number of delay days to add
/// * `adjust_to_business` - Whether to adjust to next business day
///
/// # Returns
///
/// Actual payment date
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::mbs_passthrough::delay::actual_payment_date;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let accrual_end = Date::from_calendar_date(2024, Month::January, 31).unwrap();
/// let payment_date = actual_payment_date(accrual_end, 25, false).unwrap();
/// // Payment is Feb 25, 2024
/// ```
pub fn actual_payment_date(
    accrual_end: Date,
    delay_days: u32,
    adjust_to_business: bool,
) -> Result<Date> {
    use time::Duration;

    let payment = accrual_end + Duration::days(delay_days as i64);

    if adjust_to_business {
        // Simple weekend adjustment (Following convention)
        let weekday = payment.weekday();
        let adjustment = match weekday {
            time::Weekday::Saturday => 2,
            time::Weekday::Sunday => 1,
            _ => 0,
        };
        Ok(payment + Duration::days(adjustment))
    } else {
        Ok(payment)
    }
}

/// Calculate payment date with calendar adjustment.
///
/// Uses a specific calendar for business day adjustment.
///
/// # Arguments
///
/// * `accrual_end` - End date of the accrual period
/// * `agency` - Agency program (determines delay)
/// * `calendar_id` - Calendar identifier for business day adjustment
/// * `bdc` - Business day convention
///
/// # Returns
///
/// Adjusted payment date
pub fn payment_date_with_calendar(
    accrual_end: Date,
    agency: AgencyProgram,
    calendar_id: Option<&str>,
    bdc: BusinessDayConvention,
) -> Result<Date> {
    use time::Duration;

    let delay = agency.payment_lag_days();
    let raw_payment = accrual_end + Duration::days(delay as i64);

    // Use holiday calendar when provided; fall back to weekend-only adjustment.
    if let Some(cal_id) = calendar_id {
        if let Some(cal) = calendar_by_id(cal_id) {
            return finstack_core::dates::adjust(raw_payment, bdc, cal);
        }
    }

    // Weekend-only fallback
    match bdc {
        BusinessDayConvention::Following => {
            let weekday = raw_payment.weekday();
            let adjustment = match weekday {
                time::Weekday::Saturday => 2,
                time::Weekday::Sunday => 1,
                _ => 0,
            };
            Ok(raw_payment + Duration::days(adjustment))
        }
        BusinessDayConvention::ModifiedFollowing => {
            // Same as Following, but roll back if crosses month boundary
            let weekday = raw_payment.weekday();
            let adjustment = match weekday {
                time::Weekday::Saturday => 2,
                time::Weekday::Sunday => 1,
                _ => 0,
            };
            let adjusted = raw_payment + Duration::days(adjustment);
            if adjusted.month() != raw_payment.month() {
                // Roll back to previous business day
                let back_adjustment = match weekday {
                    time::Weekday::Saturday => -1,
                    time::Weekday::Sunday => -2,
                    _ => 0,
                };
                Ok(raw_payment + Duration::days(back_adjustment))
            } else {
                Ok(adjusted)
            }
        }
        _ => Ok(raw_payment),
    }
}

/// Generate payment schedule with delays for a series of accrual periods.
///
/// # Arguments
///
/// * `accrual_ends` - Slice of accrual period end dates
/// * `agency` - Agency program (determines delay)
///
/// # Returns
///
/// Vector of (accrual_end, payment_date) pairs
pub fn payment_schedule(accrual_ends: &[Date], agency: AgencyProgram) -> Result<Vec<(Date, Date)>> {
    let delay = agency.payment_lag_days();

    accrual_ends
        .iter()
        .map(|&accrual_end| {
            let payment = actual_payment_date(accrual_end, delay, false)?;
            Ok((accrual_end, payment))
        })
        .collect()
}

/// Calculate the time value impact of payment delay.
///
/// Returns the discount factor adjustment for the delay period.
///
/// # Arguments
///
/// * `delay_days` - Number of delay days
/// * `rate` - Annualized discount rate
///
/// # Returns
///
/// Discount factor for the delay (< 1.0 for positive rates)
pub fn delay_discount_factor(delay_days: u32, rate: f64) -> f64 {
    let years = delay_days as f64 / 365.0;
    (-rate * years).exp()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_payment_lag_days() {
        assert_eq!(payment_lag_days(AgencyProgram::Fnma), 55);
        assert_eq!(payment_lag_days(AgencyProgram::Fhlmc), 55);
        assert_eq!(payment_lag_days(AgencyProgram::Gnma), 50);
        assert_eq!(payment_lag_days(AgencyProgram::GnmaI), 14);
        assert_eq!(payment_lag_days(AgencyProgram::GnmaII), 50);
    }

    #[test]
    fn test_actual_payment_date() {
        // January 31 + 25 days = February 25
        let accrual_end = Date::from_calendar_date(2024, Month::January, 31).expect("valid date");
        let payment = actual_payment_date(accrual_end, 25, false).expect("valid date");

        assert_eq!(payment.month(), Month::February);
        assert_eq!(payment.day(), 25);
    }

    #[test]
    fn test_actual_payment_date_weekend_adjustment() {
        // Find a date where +25 lands on a weekend
        // Jan 6, 2024 is Saturday. So accrual end Dec 12, 2023 + 25 = Jan 6 (Saturday)
        let accrual_end = Date::from_calendar_date(2023, Month::December, 12).expect("valid date");
        let payment_no_adjust = actual_payment_date(accrual_end, 25, false).expect("valid date");
        let payment_adjusted = actual_payment_date(accrual_end, 25, true).expect("valid date");

        // Without adjustment: Jan 6, 2024 (Saturday)
        assert_eq!(payment_no_adjust.day(), 6);
        // With adjustment: Jan 8, 2024 (Monday)
        assert_eq!(payment_adjusted.day(), 8);
    }

    #[test]
    fn test_payment_schedule() {
        let accrual_ends = vec![
            Date::from_calendar_date(2024, Month::January, 31).expect("valid"),
            Date::from_calendar_date(2024, Month::February, 29).expect("valid"),
            Date::from_calendar_date(2024, Month::March, 31).expect("valid"),
        ];

        let schedule = payment_schedule(&accrual_ends, AgencyProgram::Fnma).expect("valid");

        assert_eq!(schedule.len(), 3);

        // First payment: Jan 31 + 55 = Mar 26 (2024 is a leap year)
        assert_eq!(schedule[0].0, accrual_ends[0]);
        assert_eq!(schedule[0].1.month(), Month::March);
        assert_eq!(schedule[0].1.day(), 26);
    }

    #[test]
    fn test_delay_discount_factor() {
        // 25 days at 5% rate
        let df = delay_discount_factor(25, 0.05);

        // Should be slightly less than 1.0
        assert!(df < 1.0);
        assert!(df > 0.99);

        // Approximate: exp(-0.05 * 25/365) ≈ 0.9966
        assert!((df - 0.9966).abs() < 0.001);
    }

    #[test]
    fn test_payment_date_with_calendar() {
        let accrual_end = Date::from_calendar_date(2024, Month::January, 31).expect("valid");

        // FNMA with Following convention
        let payment = payment_date_with_calendar(
            accrual_end,
            AgencyProgram::Fnma,
            None,
            BusinessDayConvention::Following,
        )
        .expect("valid");

        // Jan 31 + 55 = Mar 26, 2024 (Tuesday, no weekend adjustment needed)
        assert_eq!(payment.month(), Month::March);
        assert_eq!(payment.day(), 26);
    }
}
