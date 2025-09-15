//! Extension traits providing convenience methods on `time::Date` and `time::OffsetDateTime`.
//!
//! These helpers are intentionally lightweight and do not allocate.

#![allow(clippy::wrong_self_convention)]

use crate::dates::calendar::core::{seek_business_day, MAX_BUSINESS_DAY_SEARCH_DAYS};
use crate::dates::periods::{days_in_month, FiscalConfig};
use time::{Date, Duration, OffsetDateTime, Weekday};

/// Convenience extensions for [`time::Date`].
pub trait DateExt: Sized {
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
    /// holidays, use [`DateExt::add_business_days`] with a `HolidayCalendar`.
    /// Positive `n` moves forward, negative `n` moves backward. Zero returns
    /// the input unchanged.
    fn add_weekdays(self, n: i32) -> Self;

    /// Add / subtract a number of **business days** (`n`) to the date using
    /// the provided `calendar` for holiday lookup.
    ///
    /// This algorithm skips weekends AND holidays according to the calendar.
    /// Positive `n` moves forward, negative `n` moves backward. Zero returns
    /// the input unchanged.
    ///
    /// Returns an error if no business day is found within the bounded search window.
    ///
    /// Example:
    /// ```
    /// use finstack_core::dates::{Date, DateExt};
    /// use finstack_core::dates::calendar::Target2;
    /// use time::Month;
    /// let cal = Target2;
    /// let start = Date::from_calendar_date(2025, Month::June, 27).unwrap(); // Friday
    /// let next = start.add_business_days(3, &cal).unwrap();
    /// assert_eq!(next, Date::from_calendar_date(2025, Month::July, 2).unwrap());
    /// ```
    fn add_business_days<C: crate::dates::calendar::HolidayCalendar>(
        self,
        n: i32,
        cal: &C,
    ) -> crate::Result<Self>;

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
            date += Duration::days(step as i64);
            if !date.is_weekend() {
                n -= step;
            }
        }
        date
    }

    fn add_business_days<C: crate::dates::calendar::HolidayCalendar>(
        self,
        n: i32,
        cal: &C,
    ) -> crate::Result<Self> {
        if n == 0 {
            return Ok(self);
        }

        let step = if n > 0 { 1 } else { -1 };
        let mut current = self;
        for _ in 0..n.unsigned_abs() {
            // move at least one day in the desired direction, then seek to a business day
            let start = current + Duration::days(step as i64);
            current = seek_business_day(start, step, MAX_BUSINESS_DAY_SEARCH_DAYS, cal, "BusinessDayAddition", self)?;
        }
        Ok(current)
    }

    fn is_business_day<C: crate::dates::calendar::HolidayCalendar>(self, cal: &C) -> bool {
        cal.is_business_day(self)
    }

    fn next_imm(self) -> Self {
        crate::dates::next_imm(self)
    }
}

/// Convenience extensions for [`time::OffsetDateTime`].
pub trait OffsetDateTimeExt: Sized {
    /// See [`DateExt::is_weekend`].
    fn is_weekend(self) -> bool;

    /// See [`DateExt::quarter`].
    fn quarter(self) -> u8;

    /// See [`DateExt::fiscal_year`].
    fn fiscal_year(self, config: FiscalConfig) -> i32;

    /// See [`DateExt::add_weekdays`].
    fn add_weekdays(self, n: i32) -> Self;

    /// See [`DateExt::add_business_days`].
    fn add_business_days<C: crate::dates::calendar::HolidayCalendar>(
        self,
        n: i32,
        cal: &C,
    ) -> crate::Result<Self>;

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

    fn add_business_days<C: crate::dates::calendar::HolidayCalendar>(
        self,
        n: i32,
        cal: &C,
    ) -> crate::Result<Self> {
        let new_date = self.date().add_business_days(n, cal)?;
        Ok(self.replace_date(new_date))
    }

    fn is_business_day<C: crate::dates::calendar::HolidayCalendar>(self, cal: &C) -> bool {
        self.date().is_business_day(cal)
    }

    fn next_imm(self) -> Self {
        let new_date = self.date().next_imm();
        self.replace_date(new_date)
    }
}

/// Iterator over business days between two bounds using a `HolidayCalendar`.
///
/// Forward iteration yields dates in [start, end). For reverse scans, prefer
/// constructing with start/end swapped and iterating forward, or add a simple
/// `.rev()` on a collected Vec if needed.
#[derive(Clone, Debug)]
pub struct BusinessDayIter<'a, C: crate::dates::calendar::HolidayCalendar + ?Sized> {
    current: Date,
    end: Date,
    cal: &'a C,
}

impl<'a, C: crate::dates::calendar::HolidayCalendar + ?Sized> BusinessDayIter<'a, C> {
    /// Create a forward iterator over business days in [start, end).
    pub fn new(start: Date, end: Date, cal: &'a C) -> Self {
        Self {
            current: start,
            end,
            cal,
        }
    }
}

impl<C: crate::dates::calendar::HolidayCalendar + ?Sized> Iterator for BusinessDayIter<'_, C> {
    type Item = Date;
    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.end {
            let d = self.current;
            self.current += Duration::days(1);
            if self.cal.is_business_day(d) {
                return Some(d);
            }
        }
        None
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

    #[test]
    fn test_add_business_days_forward() {
        use crate::dates::calendar::Target2;

        let cal = Target2;

        // Start on Friday, add 3 business days should land on Wednesday (skip weekend)
        let friday = make_date(2025, 6, 27);
        let result = friday.add_business_days(3, &cal).unwrap();
        assert_eq!(result, make_date(2025, 7, 2)); // Wednesday
    }

    #[test]
    fn test_add_business_days_backward() {
        use crate::dates::calendar::Target2;

        let cal = Target2;

        // Start on Monday, subtract 3 business days should land on Wednesday previous week
        let monday = make_date(2025, 6, 30);
        let result = monday.add_business_days(-3, &cal).unwrap();
        assert_eq!(result, make_date(2025, 6, 25)); // Wednesday
    }

    #[test]
    fn test_add_business_days_zero() {
        use crate::dates::calendar::Target2;

        let cal = Target2;
        let date = make_date(2025, 6, 27);
        let result = date.add_business_days(0, &cal).unwrap();
        assert_eq!(result, date);
    }

    #[test]
    fn test_add_business_days_with_holidays() {
        use crate::dates::calendar::HolidayCalendar;
        use crate::dates::calendar::Target2;

        let cal = Target2;

        // Test around a known holiday period (Christmas/New Year)
        // December 24, 2024 is Tuesday
        let christmas_eve = make_date(2024, 12, 24);
        let result = christmas_eve.add_business_days(1, &cal).unwrap();

        // Should skip Christmas Day (Dec 25), Boxing Day (Dec 26), and weekends
        // Landing on the next available business day
        assert!(result > christmas_eve);
        assert!(cal.is_business_day(result));
    }

    #[test]
    fn test_add_business_days_offset_datetime() {
        use crate::dates::calendar::Target2;

        let cal = Target2;

        // Create OffsetDateTime
        let dt = make_date(2025, 6, 27)
            .with_hms(10, 30, 0)
            .unwrap()
            .assume_utc();

        let result = dt.add_business_days(3, &cal).unwrap();
        assert_eq!(result.date(), make_date(2025, 7, 2));
        assert_eq!(result.time(), dt.time()); // Time should be preserved
    }

    #[test]
    fn test_add_business_days_error_on_all_holidays() {
        // A calendar that marks every day as a holiday to trigger bounded search failure
        struct AllHolidaysCal;
        impl crate::dates::calendar::HolidayCalendar for AllHolidaysCal {
            fn is_holiday(&self, _date: Date) -> bool {
                true
            }
        }

        let cal = AllHolidaysCal;
        let start = make_date(2025, 1, 1);
        let err = start.add_business_days(1, &cal).unwrap_err();
        match err {
            crate::Error::Input(crate::error::InputError::AdjustmentFailed {
                max_days, ..
            }) => {
                assert_eq!(max_days, MAX_BUSINESS_DAY_SEARCH_DAYS);
            }
            other => panic!("Expected AdjustmentFailed error, got {:?}", other),
        }
    }
}
