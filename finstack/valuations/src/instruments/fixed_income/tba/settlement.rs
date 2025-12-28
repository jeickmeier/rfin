//! TBA settlement conventions.
//!
//! SIFMA TBA settlement follows standardized conventions for
//! notification and settlement dates.

use finstack_core::dates::Date;
use finstack_core::Result;
use time::{Duration, Month, Weekday};

/// TBA settlement information.
#[derive(Clone, Debug)]
pub struct TbaSettlementDates {
    /// Settlement month
    pub settlement_month: Date,
    /// Notification date (48-hour rule)
    pub notification_date: Date,
    /// Good delivery (settlement) date
    pub settlement_date: Date,
}

/// Calculate TBA settlement dates for a given month.
///
/// SIFMA TBA settlement follows these conventions:
/// - Settlement occurs in the third week of the month
/// - Notification deadline is 48 hours before settlement
///
/// # Arguments
///
/// * `year` - Settlement year
/// * `month` - Settlement month (1-12)
pub fn calculate_settlement_dates(year: i32, month: u8) -> Result<TbaSettlementDates> {
    let month_enum =
        Month::try_from(month).map_err(|e| finstack_core::Error::Validation(e.to_string()))?;

    // Find the third Wednesday of the month (typical TBA settlement)
    let first_of_month = Date::from_calendar_date(year, month_enum, 1)
        .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;

    // Find first Wednesday
    let days_until_wednesday =
        (Weekday::Wednesday as i32 - first_of_month.weekday() as i32 + 7) % 7;
    let first_wednesday = first_of_month + Duration::days(days_until_wednesday as i64);

    // Third Wednesday is 14 days later
    let settlement_date = first_wednesday + Duration::days(14);

    // Notification is 2 business days before settlement (48-hour rule)
    // For simplicity, we'll use calendar days minus weekends
    let notification_date = subtract_business_days(settlement_date, 2)?;

    // Month reference (first of month)
    let settlement_month = first_of_month;

    Ok(TbaSettlementDates {
        settlement_month,
        notification_date,
        settlement_date,
    })
}

/// Calculate the drop date (last trading day for a TBA month).
///
/// The drop date is typically 2 business days before notification.
pub fn calculate_drop_date(year: i32, month: u8) -> Result<Date> {
    let dates = calculate_settlement_dates(year, month)?;
    subtract_business_days(dates.notification_date, 2)
}

/// Subtract business days from a date.
fn subtract_business_days(date: Date, days: u32) -> Result<Date> {
    let mut result = date;
    let mut remaining = days;

    while remaining > 0 {
        result -= Duration::days(1);
        match result.weekday() {
            Weekday::Saturday | Weekday::Sunday => continue,
            _ => remaining -= 1,
        }
    }

    Ok(result)
}

/// Get the next TBA settlement month from a given date.
pub fn next_settlement_month(as_of: Date) -> Result<(i32, u8)> {
    let year = as_of.year();
    let month = as_of.month() as u8;

    // Check if current month's settlement is still tradeable
    let current_settlement = calculate_settlement_dates(year, month)?;
    let drop_date = subtract_business_days(current_settlement.notification_date, 2)?;

    if as_of <= drop_date {
        Ok((year, month))
    } else {
        // Roll to next month
        if month == 12 {
            Ok((year + 1, 1))
        } else {
            Ok((year, month + 1))
        }
    }
}

/// Get the roll date between two settlement months.
///
/// The roll typically occurs around the notification date of the
/// front-month settlement.
pub fn get_roll_date(front_year: i32, front_month: u8) -> Result<Date> {
    let front_dates = calculate_settlement_dates(front_year, front_month)?;
    // Roll typically occurs 2-3 days before notification
    subtract_business_days(front_dates.notification_date, 3)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_settlement_dates() {
        let dates = calculate_settlement_dates(2024, 3).expect("valid dates");

        // March 2024's third Wednesday is March 20
        assert_eq!(dates.settlement_date.month(), Month::March);
        assert_eq!(dates.settlement_date.year(), 2024);

        // Settlement should be on a Wednesday
        assert_eq!(dates.settlement_date.weekday(), Weekday::Wednesday);
    }

    #[test]
    fn test_notification_before_settlement() {
        let dates = calculate_settlement_dates(2024, 3).expect("valid dates");

        // Notification should be before settlement
        assert!(dates.notification_date < dates.settlement_date);

        // Should be a business day (not weekend)
        assert!(
            dates.notification_date.weekday() != Weekday::Saturday
                && dates.notification_date.weekday() != Weekday::Sunday
        );
    }

    #[test]
    fn test_drop_date() {
        let drop = calculate_drop_date(2024, 3).expect("valid date");
        let dates = calculate_settlement_dates(2024, 3).expect("valid dates");

        // Drop should be before notification
        assert!(drop < dates.notification_date);
    }

    #[test]
    fn test_next_settlement_month() {
        // Early in month, should return current month
        let early = Date::from_calendar_date(2024, Month::March, 1).expect("valid");
        let (year, month) = next_settlement_month(early).expect("valid");
        assert_eq!(year, 2024);
        assert_eq!(month, 3);

        // Late in month, should return next month
        let late = Date::from_calendar_date(2024, Month::March, 25).expect("valid");
        let (year2, month2) = next_settlement_month(late).expect("valid");
        assert_eq!(year2, 2024);
        assert_eq!(month2, 4);
    }

    #[test]
    fn test_december_rollover() {
        // December should roll to January next year
        let late_dec = Date::from_calendar_date(2024, Month::December, 25).expect("valid");
        let (year, month) = next_settlement_month(late_dec).expect("valid");
        assert_eq!(year, 2025);
        assert_eq!(month, 1);
    }
}
