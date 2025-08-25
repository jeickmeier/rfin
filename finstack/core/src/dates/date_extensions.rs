//! Extension traits providing convenience methods on `time::Date` and `time::OffsetDateTime`.
//!
//! These helpers are intentionally lightweight – they do **not** allocate and are fully
//! `no_std` compatible.  More advanced calendar-aware variants will be added in later
//! pull-requests once the holiday calendar machinery is available.

#![allow(clippy::wrong_self_convention, clippy::assign_op_pattern)]

use time::{Date, Duration, OffsetDateTime, Weekday};

/// Convenience extensions for [`time::Date`].
pub trait DateExt {
    /// Returns true if the date falls on a weekend (**Saturday** or **Sunday**).
    fn is_weekend(self) -> bool;

    /// Calendar quarter of the date (1‥=4).
    fn quarter(self) -> u8;

    /// Fiscal year corresponding to the date.
    ///
    /// Currently this is identical to the calendar year.  A configurable start
    /// month will be added in a future iteration once more requirements are
    /// clear.
    fn fiscal_year(self) -> i32;

    /// Add / subtract a number of **business days** (`n`) to the date.
    ///
    /// The naive weekend-only algorithm skips Saturdays & Sundays.  Positive
    /// `n` moves forward, negative `n` moves backward.  Zero returns the input
    /// unchanged.
    fn add_business_days(self, n: i32) -> Self;

    /// Returns `true` if the date is a business day according to the provided
    /// `calendar` (see [`crate::dates::calendar::HolidayCalendar`]).
    ///
    /// This is a thin convenience wrapper around
    /// [`HolidayCalendar::is_business_day`], enabling fluent method-style
    /// calls:
    /// ```
    /// use finstack_core::dates::DateExt;
    /// use finstack_core::dates::calendars::Gblo;
    /// use time::Date;
    ///
    /// let cal = Gblo::new();
    /// let d = Date::from_calendar_date(2025, time::Month::March, 14).unwrap();
    /// assert!(d.is_business_day(&cal));
    /// ```
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

    fn fiscal_year(self) -> i32 {
        self.year()
    }

    fn add_business_days(self, mut n: i32) -> Self {
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
    fn fiscal_year(self) -> i32;

    /// See [`DateExt::add_business_days`].
    fn add_business_days(self, n: i32) -> Self;

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

    fn fiscal_year(self) -> i32 {
        self.date().fiscal_year()
    }

    fn add_business_days(self, n: i32) -> Self {
        let new_date = self.date().add_business_days(n);
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
    fn test_add_business_days_forward() {
        let start = make_date(2025, 6, 27); // Friday
        let result = start.add_business_days(3);
        assert_eq!(result, make_date(2025, 7, 2)); // Fri +3bd = Wed (skip weekend)
    }

    #[test]
    fn test_add_business_days_backward() {
        let start = make_date(2025, 6, 29); // Sunday
        let result = start.add_business_days(-2);
        assert_eq!(result, make_date(2025, 6, 26)); // Sun -2bd = Thu (skip weekend)
    }
}
