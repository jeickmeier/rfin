//! Payment schedule generation for commodity swaps.
//!
//! Provides utilities for generating payment dates based on frequency,
//! business day conventions, and calendar adjustments.

use finstack_core::dates::{BusinessDayConvention, Date, Tenor, TenorUnit};
use finstack_core::Result;
use time::Duration;

/// Generate payment schedule for a commodity swap.
///
/// # Arguments
///
/// * `start_date` - Start date of the swap
/// * `end_date` - End date of the swap
/// * `frequency` - Payment frequency as a Tenor (e.g., 1M, 3M, 6M)
/// * `bdc` - Business day convention for date adjustments
/// * `calendar_id` - Optional calendar ID for holiday adjustments
/// * `as_of` - Valuation date (for filtering past dates if needed)
///
/// # Returns
///
/// Vector of payment dates from start to end.
pub fn generate_payment_schedule(
    start_date: Date,
    end_date: Date,
    frequency: Tenor,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
    _as_of: Date,
) -> Result<Vec<Date>> {
    let mut dates = Vec::new();

    // Calculate period length based on tenor unit
    match frequency.unit {
        TenorUnit::Days => {
            let days_per_period = frequency.count as i64;
            let mut current = start_date;
            while current <= end_date {
                let adjusted = adjust_for_bdc(current, bdc, calendar_id)?;
                if adjusted <= end_date && adjusted > start_date {
                    dates.push(adjusted);
                }
                current += Duration::days(days_per_period);
            }
        }
        TenorUnit::Weeks => {
            let days_per_period = frequency.count as i64 * 7;
            let mut current = start_date;
            while current <= end_date {
                let adjusted = adjust_for_bdc(current, bdc, calendar_id)?;
                if adjusted <= end_date && adjusted > start_date {
                    dates.push(adjusted);
                }
                current += Duration::days(days_per_period);
            }
        }
        TenorUnit::Months => {
            let months_per_period = frequency.count as i32;
            let mut period_count = 1; // Start at 1 to skip start date

            loop {
                let next_date = add_months(start_date, period_count * months_per_period)?;

                if next_date > end_date {
                    break;
                }

                let adjusted = adjust_for_bdc(next_date, bdc, calendar_id)?;
                if adjusted <= end_date {
                    dates.push(adjusted);
                }

                period_count += 1;

                // Safety: prevent infinite loops
                if period_count > 1000 {
                    break;
                }
            }

            // Ensure we include end date if not already included
            let adjusted_end = adjust_for_bdc(end_date, bdc, calendar_id)?;
            if dates.is_empty() || dates.last() != Some(&adjusted_end) {
                dates.push(adjusted_end);
            }
        }
        TenorUnit::Years => {
            let years_per_period = frequency.count as i32;
            let mut period_count = 1;

            loop {
                let next_date = add_months(start_date, period_count * years_per_period * 12)?;

                if next_date > end_date {
                    break;
                }

                let adjusted = adjust_for_bdc(next_date, bdc, calendar_id)?;
                if adjusted <= end_date {
                    dates.push(adjusted);
                }

                period_count += 1;

                if period_count > 100 {
                    break;
                }
            }

            let adjusted_end = adjust_for_bdc(end_date, bdc, calendar_id)?;
            if dates.is_empty() || dates.last() != Some(&adjusted_end) {
                dates.push(adjusted_end);
            }
        }
    }

    // Sort and deduplicate
    dates.sort();
    dates.dedup();

    Ok(dates)
}

/// Add months to a date, handling end-of-month cases.
fn add_months(date: Date, months: i32) -> Result<Date> {
    let (year, month, day) = (date.year(), date.month(), date.day());

    let total_months = year * 12 + (month as i32 - 1) + months;
    let new_year = total_months / 12;
    let new_month_num = (total_months % 12) + 1;

    let new_month = match new_month_num {
        1 => time::Month::January,
        2 => time::Month::February,
        3 => time::Month::March,
        4 => time::Month::April,
        5 => time::Month::May,
        6 => time::Month::June,
        7 => time::Month::July,
        8 => time::Month::August,
        9 => time::Month::September,
        10 => time::Month::October,
        11 => time::Month::November,
        12 => time::Month::December,
        _ => unreachable!(),
    };

    // Handle end-of-month: cap day at month's max days
    let max_day = days_in_month(new_year, new_month);
    let new_day = day.min(max_day);

    Date::from_calendar_date(new_year, new_month, new_day).map_err(|e| {
        finstack_core::Error::Validation(format!(
            "Failed to create date: {}-{:02}-{:02}: {}",
            new_year, new_month_num, new_day, e
        ))
    })
}

/// Get the number of days in a month.
fn days_in_month(year: i32, month: time::Month) -> u8 {
    match month {
        time::Month::January => 31,
        time::Month::February => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        time::Month::March => 31,
        time::Month::April => 30,
        time::Month::May => 31,
        time::Month::June => 30,
        time::Month::July => 31,
        time::Month::August => 31,
        time::Month::September => 30,
        time::Month::October => 31,
        time::Month::November => 30,
        time::Month::December => 31,
    }
}

/// Check if a year is a leap year.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Adjust a date according to business day convention.
///
/// Note: This is a simplified implementation. In production, this would
/// use the calendar system to check for holidays.
fn adjust_for_bdc(
    date: Date,
    bdc: BusinessDayConvention,
    _calendar_id: Option<&str>,
) -> Result<Date> {
    // Simplified: only adjust for weekends
    let weekday = date.weekday();

    match bdc {
        BusinessDayConvention::Unadjusted => Ok(date),
        BusinessDayConvention::Following => {
            match weekday {
                time::Weekday::Saturday => Ok(date + Duration::days(2)),
                time::Weekday::Sunday => Ok(date + Duration::days(1)),
                _ => Ok(date),
            }
        }
        BusinessDayConvention::ModifiedFollowing => {
            let adjusted = match weekday {
                time::Weekday::Saturday => date + Duration::days(2),
                time::Weekday::Sunday => date + Duration::days(1),
                _ => date,
            };
            // Check if we crossed month boundary
            if adjusted.month() != date.month() {
                // Go back to previous business day
                let mut prev = date;
                loop {
                    prev -= Duration::days(1);
                    let wd = prev.weekday();
                    if wd != time::Weekday::Saturday && wd != time::Weekday::Sunday {
                        return Ok(prev);
                    }
                }
            }
            Ok(adjusted)
        }
        BusinessDayConvention::Preceding => {
            match weekday {
                time::Weekday::Saturday => Ok(date - Duration::days(1)),
                time::Weekday::Sunday => Ok(date - Duration::days(2)),
                _ => Ok(date),
            }
        }
        BusinessDayConvention::ModifiedPreceding => {
            let adjusted = match weekday {
                time::Weekday::Saturday => date - Duration::days(1),
                time::Weekday::Sunday => date - Duration::days(2),
                _ => date,
            };
            // Check if we crossed month boundary
            if adjusted.month() != date.month() {
                // Go forward to next business day
                let mut next = date;
                loop {
                    next += Duration::days(1);
                    let wd = next.weekday();
                    if wd != time::Weekday::Saturday && wd != time::Weekday::Sunday {
                        return Ok(next);
                    }
                }
            }
            Ok(adjusted)
        }
        // Handle other conventions as unadjusted for now
        _ => Ok(date),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn test_generate_monthly_schedule() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let end = Date::from_calendar_date(2025, Month::June, 30).expect("valid date");
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let monthly = Tenor::new(1, TenorUnit::Months);

        let schedule = generate_payment_schedule(
            start,
            end,
            monthly,
            BusinessDayConvention::Unadjusted,
            None,
            as_of,
        )
        .expect("schedule generation");

        // Should have monthly payments from Feb to Jun (5+ periods)
        assert!(!schedule.is_empty());
        assert!(schedule.len() >= 5);
    }

    #[test]
    fn test_generate_quarterly_schedule() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let end = Date::from_calendar_date(2025, Month::December, 31).expect("valid date");
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let quarterly = Tenor::new(3, TenorUnit::Months);

        let schedule = generate_payment_schedule(
            start,
            end,
            quarterly,
            BusinessDayConvention::Unadjusted,
            None,
            as_of,
        )
        .expect("schedule generation");

        // Should have 4 quarterly payments
        assert!(schedule.len() >= 4);
    }

    #[test]
    fn test_add_months() {
        let date = Date::from_calendar_date(2025, Month::January, 31).expect("valid date");
        let result = add_months(date, 1).expect("valid result");
        // February doesn't have 31 days, should be Feb 28
        assert_eq!(result.month(), Month::February);
        assert_eq!(result.day(), 28);
    }

    #[test]
    fn test_bdc_following() {
        // 2025-01-04 is a Saturday
        let saturday = Date::from_calendar_date(2025, Month::January, 4).expect("valid date");
        let adjusted =
            adjust_for_bdc(saturday, BusinessDayConvention::Following, None).expect("valid adjustment");
        // Should be Monday Jan 6
        assert_eq!(adjusted.day(), 6);
    }
}
