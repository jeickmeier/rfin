//! Extension traits providing convenience methods on `time::Date` and `time::OffsetDateTime`.
//!
//! These helpers are intentionally lightweight and do not allocate. More advanced
//! calendar-aware variants will be added in later pull-requests once the holiday
//! calendar machinery is available.

#![allow(clippy::wrong_self_convention, clippy::assign_op_pattern)]

use time::{Date, Duration, OffsetDateTime, Weekday};
use crate::dates::periods::{FiscalConfig, days_in_month};

/// Convenience extensions for [`time::Date`].
pub trait DateExt {
    /// Returns true if the date falls on a weekend (**Saturday** or **Sunday**).
    fn is_weekend(self) -> bool;

    /// Calendar quarter of the date (1‥=4).
    fn quarter(self) -> u8;

    /// Fiscal year corresponding to the date based on the provided fiscal configuration.
    ///
    /// Uses the fiscal year start month and day from `FiscalConfig` to determine
    /// which fiscal year this date belongs to.
    fn fiscal_year(self, config: FiscalConfig) -> i32;

    /// Add / subtract a number of **weekdays** (`n`) to the date.
    ///
    /// This naive algorithm only skips Saturdays & Sundays, and does NOT
    /// account for holidays. For true business day adjustments that respect
    /// holidays, use a `HolidayCalendar` with appropriate methods.
    /// Positive `n` moves forward, negative `n` moves backward. Zero returns
    /// the input unchanged.
    fn add_weekdays(self, n: i32) -> Self;

    /// Returns `true` if the date is a business day according to the provided
    /// `calendar` (see [`crate::dates::calendar::HolidayCalendar`]).
    ///
    /// This is a thin convenience wrapper around
    /// [`HolidayCalendar::is_business_day`], enabling fluent method-style
    /// calls. See repository examples under `examples/` for usage.
    fn is_business_day<C: crate::dates::calendar::HolidayCalendar>(self, cal: &C) -> bool;

    /// Returns the **next IMM date** (third Wednesday of Mar/Jun/Sep/Dec)
    /// strictly **after** `self`.
    ///
    /// Equivalent to calling [`crate::dates::next_imm`] but available as a
    /// method for improved discoverability.
    fn next_imm(self) -> Self;
}

impl DateExt for Date {
    fn is_weekend(self) -> bool {
        matches!(self.weekday(), Weekday::Saturday | Weekday::Sunday)
    }

    fn quarter(self) -> u8 {
        // SAFETY: Month is 1-12 – map to 1-4
        ((self.month() as u8 - 1) / 3) + 1
    }

    fn fiscal_year(self, config: FiscalConfig) -> i32 {
        // If fiscal year starts in January, it's just the calendar year
        if config.start_month == 1 && config.start_day == 1 {
            return self.year();
        }

        // Otherwise, we need to check if the date is before or after the fiscal year start
        let calendar_year = self.year();
        let fiscal_start_this_year = Date::from_calendar_date(
            calendar_year,
            time::Month::try_from(config.start_month).unwrap(),
            config.start_day,
        )
        .unwrap_or_else(|_| {
            // If the day doesn't exist (e.g., Feb 30), use the last day of the month
            let last_day = days_in_month(calendar_year, config.start_month);
            Date::from_calendar_date(
                calendar_year,
                time::Month::try_from(config.start_month).unwrap(),
                last_day,
            )
            .unwrap()
        });

        if self >= fiscal_start_this_year {
            // Date is on or after fiscal year start, so it belongs to the fiscal year
            // that started in this calendar year
            calendar_year + 1
        } else {
            // Date is before fiscal year start, so it belongs to the fiscal year
            // that started in the previous calendar year
            calendar_year
        }
    }

    fn add_weekdays(self, mut n: i32) -> Self {
        if n == 0 {
            return self;
        }

        let step = if n > 0 { 1 } else { -1 };
        let mut date = self;
        while n != 0 {
            // Safe unwrap: adding 1 day to a valid Date always succeeds within time range.
            date = date + Duration::days(step as i64);
            if !date.is_weekend() {
                n -= step;
            }
        }
        date
    }

    fn is_business_day<C: crate::dates::calendar::HolidayCalendar>(self, cal: &C) -> bool {
        cal.is_business_day(self)
    }

    fn next_imm(self) -> Self {
        crate::dates::next_imm(self)
    }
}

/// Convenience extensions for [`time::OffsetDateTime`].
pub trait OffsetDateTimeExt {
    /// See [`DateExt::is_weekend`].
    fn is_weekend(self) -> bool;

    /// See [`DateExt::quarter`].
    fn quarter(self) -> u8;

    /// See [`DateExt::fiscal_year`].
    fn fiscal_year(self, config: FiscalConfig) -> i32;

    /// See [`DateExt::add_weekdays`].
    fn add_weekdays(self, n: i32) -> Self;

    /// See [`DateExt::is_business_day`].
    fn is_business_day<C: crate::dates::calendar::HolidayCalendar>(self, cal: &C) -> bool;

    /// See [`DateExt::next_imm`].
    fn next_imm(self) -> Self;
}

impl OffsetDateTimeExt for OffsetDateTime {
    fn is_weekend(self) -> bool {
        self.date().is_weekend()
    }

    fn quarter(self) -> u8 {
        self.date().quarter()
    }

    fn fiscal_year(self, config: FiscalConfig) -> i32 {
        self.date().fiscal_year(config)
    }

    fn add_weekdays(self, n: i32) -> Self {
        let new_date = self.date().add_weekdays(n);
        self.replace_date(new_date)
    }

    fn is_business_day<C: crate::dates::calendar::HolidayCalendar>(self, cal: &C) -> bool {
        self.date().is_business_day(cal)
    }

    fn next_imm(self) -> Self {
        let new_date = self.date().next_imm();
        self.replace_date(new_date)
    }
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use time::Date;

    fn make_date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
    }

    #[test]
    fn test_is_weekend() {
        let sat = make_date(2025, 6, 28);
        let sun = make_date(2025, 6, 29);
        let fri = make_date(2025, 6, 27);
        assert!(sat.is_weekend());
        assert!(sun.is_weekend());
        assert!(!fri.is_weekend());
    }

    #[test]
    fn test_quarter() {
        assert_eq!(make_date(2025, 1, 1).quarter(), 1);
        assert_eq!(make_date(2025, 4, 15).quarter(), 2);
        assert_eq!(make_date(2025, 8, 1).quarter(), 3);
        assert_eq!(make_date(2025, 11, 30).quarter(), 4);
    }

    #[test]
    fn test_add_weekdays_forward() {
        let start = make_date(2025, 6, 27); // Friday
        let result = start.add_weekdays(3);
        assert_eq!(result, make_date(2025, 7, 2)); // Fri +3 weekdays = Wed (skip weekend)
    }

    #[test]
    fn test_add_weekdays_backward() {
        let start = make_date(2025, 6, 29); // Sunday
        let result = start.add_weekdays(-2);
        assert_eq!(result, make_date(2025, 6, 26)); // Sun -2 weekdays = Thu (skip weekend)
    }

    #[test]
    fn test_fiscal_year_calendar_year() {
        let date = make_date(2025, 6, 15);
        let config = FiscalConfig::calendar_year();
        assert_eq!(date.fiscal_year(config), 2025);
    }

    #[test]
    fn test_fiscal_year_us_federal() {
        let config = FiscalConfig::us_federal(); // October 1 start
        
        // Date before fiscal year start (e.g., September) belongs to previous FY
        let sept_date = make_date(2024, 9, 15);
        assert_eq!(sept_date.fiscal_year(config), 2024);
        
        // Date on or after fiscal year start belongs to current FY
        let oct_date = make_date(2024, 10, 1);
        assert_eq!(oct_date.fiscal_year(config), 2025);
        
        let dec_date = make_date(2024, 12, 15);
        assert_eq!(dec_date.fiscal_year(config), 2025);
    }

    #[test]
    fn test_fiscal_year_uk() {
        let config = FiscalConfig::uk(); // April 6 start
        
        // Date before fiscal year start belongs to previous FY
        let march_date = make_date(2025, 3, 15);
        assert_eq!(march_date.fiscal_year(config), 2025);
        
        // Date on or after fiscal year start belongs to current FY
        let april_date = make_date(2025, 4, 6);
        assert_eq!(april_date.fiscal_year(config), 2026);
        
        let may_date = make_date(2025, 5, 15);
        assert_eq!(may_date.fiscal_year(config), 2026);
    }
}
