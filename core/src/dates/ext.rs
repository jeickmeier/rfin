//! Extension traits providing convenience methods on `time::Date` and `time::OffsetDateTime`.
//!
//! These helpers are intentionally lightweight – they do **not** allocate and are fully
//! `no_std` compatible.  More advanced calendar-aware variants will be added in later
//! pull-requests once the holiday calendar machinery is available.

#![allow(clippy::many_single_char_names)]
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
