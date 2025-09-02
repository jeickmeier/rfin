//! Day-count convention algorithms (ACT/360, ACT/365F, 30/360, 30E/360, ACT/ACT).
//!
//! The implementation follows the ISDA definitions where applicable and is **panic-free**.
//! All helpers avoid heap allocation.
//!
//! # Examples
//! ```
//! use finstack_core::dates::{Date, DayCount};
//! use time::Month;
//!
//! let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let end   = Date::from_calendar_date(2026, Month::January, 1).unwrap();
//!
//! let yf = DayCount::ActAct.year_fraction(start, end).unwrap();
//! assert!((yf - 1.0).abs() < 1e-9);
//! ```

#![allow(clippy::many_single_char_names)]

use core::cmp::Ordering;
use time::{Date, Duration, Month};

use crate::error::InputError;
use crate::dates::calendar::HolidayCalendar;

/// Supported day-count conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum DayCount {
    /// Actual / 360 — year fraction = actual days ÷ 360.
    Act360,
    /// Actual / 365F — year fraction = actual days ÷ 365 (fixed).
    Act365F,
    /// Actual / 365L (Actual/365 Leap or AFB) — denominator varies based on leap year logic.
    Act365L,
    /// 30U/360 (US Bond Basis).
    Thirty360,
    /// 30E/360 (European).
    ThirtyE360,
    /// Actual / Actual (ISDA variant).
    ActAct,
    /// Bus/252 — business days ÷ 252 (requires holiday calendar).
    Bus252,
}

impl DayCount {
    /// Return the day count between `start` (inclusive) and `end` (exclusive).
    ///
    /// The output follows the specific convention rules and is **always ≥ 0**.
    /// 
    /// # Note
    /// For `Bus/252`, use [`DayCount::business_days`] with a holiday calendar instead.
    #[doc(hidden)]
    pub fn days(self, start: Date, end: Date) -> crate::Result<i32> {
        match start.cmp(&end) {
            Ordering::Greater => Err(InputError::InvalidDateRange.into()),
            Ordering::Equal => Ok(0),
            Ordering::Less => match self {
                DayCount::Act360 | DayCount::Act365F | DayCount::Act365L | DayCount::ActAct => {
                    let total_days = (end - start).whole_days();
                    Ok(total_days as i32)
                }
                DayCount::Thirty360 => Ok(days_30_360(start, end, Thirty360Convention::Us)),
                DayCount::ThirtyE360 => Ok(days_30_360(start, end, Thirty360Convention::European)),
                DayCount::Bus252 => Err(InputError::Invalid.into()),
            },
        }
    }

    /// Compute the year fraction between `start` and `end` per this convention.
    /// 
    /// # Note
    /// For `Bus/252`, use [`DayCount::year_fraction_with_calendar`] with a holiday calendar instead.
    pub fn year_fraction(self, start: Date, end: Date) -> crate::Result<f64> {
        match start.cmp(&end) {
            Ordering::Greater => Err(InputError::InvalidDateRange.into()),
            Ordering::Equal => Ok(0.0),
            Ordering::Less => {
                let yf = match self {
                    DayCount::Act360 => {
                        let days = (end - start).whole_days() as f64;
                        days / 360.0
                    }
                    DayCount::Act365F => {
                        let days = (end - start).whole_days() as f64;
                        days / 365.0
                    }
                    DayCount::Act365L => year_fraction_act_365l(start, end),
                    DayCount::Thirty360 => {
                        let days = days_30_360(start, end, Thirty360Convention::Us) as f64;
                        days / 360.0
                    }
                    DayCount::ThirtyE360 => {
                        let days = days_30_360(start, end, Thirty360Convention::European) as f64;
                        days / 360.0
                    }
                    DayCount::ActAct => year_fraction_act_act_isda(start, end),
                    DayCount::Bus252 => return Err(InputError::Invalid.into()),
                };
                Ok(yf)
            }
        }
    }

    /// Count business days between `start` (inclusive) and `end` (exclusive) using the given calendar.
    /// 
    /// This is primarily used for `Bus/252` day count convention but can be used with any calendar.
    pub fn business_days<C: HolidayCalendar + ?Sized>(
        self,
        start: Date,
        end: Date,
        calendar: &C,
    ) -> crate::Result<i32> {
        match start.cmp(&end) {
            Ordering::Greater => Err(InputError::InvalidDateRange.into()),
            Ordering::Equal => Ok(0),
            Ordering::Less => match self {
                DayCount::Bus252 => Ok(count_business_days(start, end, calendar)),
                _ => {
                    // For other conventions, business_days should just return regular days
                    self.days(start, end)
                }
            },
        }
    }

    /// Compute the year fraction between `start` and `end` using the given calendar.
    /// 
    /// This method is required for `Bus/252` and can be used with other conventions.
    pub fn year_fraction_with_calendar<C: HolidayCalendar + ?Sized>(
        self,
        start: Date,
        end: Date,
        calendar: &C,
    ) -> crate::Result<f64> {
        match start.cmp(&end) {
            Ordering::Greater => Err(InputError::InvalidDateRange.into()),
            Ordering::Equal => Ok(0.0),
            Ordering::Less => {
                let yf = match self {
                    DayCount::Bus252 => {
                        let biz_days = count_business_days(start, end, calendar) as f64;
                        biz_days / 252.0
                    }
                    _ => {
                        // For other conventions, delegate to regular year_fraction
                        return self.year_fraction(start, end);
                    }
                };
                Ok(yf)
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// 30/360 generalized helper
// -------------------------------------------------------------------------------------------------
/// 30/360 day-count variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thirty360Convention {
    /// 30U/360 (US Bond Basis).
    Us,
    /// 30E/360 (European).
    European,
}

/// Compute day count between `start` (inclusive) and `end` (exclusive) under a 30/360 convention.
///
/// Precondition: `start <= end`. If violated, the returned value will be negative.
/// This helper is panic-free and allocation-free.
#[inline]
pub(crate) fn days_30_360(start: Date, end: Date, convention: Thirty360Convention) -> i32 {
    let (y1, m1, d1) = (start.year(), start.month() as i32, start.day() as i32);
    let (y2, m2, d2) = (end.year(), end.month() as i32, end.day() as i32);

    let d1_adj = if d1 == 31 { 30 } else { d1 };
    let d2_adj = match convention {
        Thirty360Convention::Us => {
            if d2 == 31 && d1_adj == 30 {
                30
            } else {
                d2
            }
        }
        Thirty360Convention::European => {
            if d2 == 31 {
                30
            } else {
                d2
            }
        }
    };

    (y2 - y1) * 360 + (m2 - m1) * 30 + (d2_adj - d1_adj)
}

// (Wrappers removed in favor of the public `days_30_360` with `Thirty360Convention`.)

// -------------------------------------------------------------------------------------------------
// ACT/ACT (ISDA) helper
// -------------------------------------------------------------------------------------------------
fn year_fraction_act_act_isda(start: Date, end: Date) -> f64 {
    if start == end {
        return 0.0;
    }

    if start.year() == end.year() {
        let denom = days_in_year(start.year()) as f64;
        let days = (end - start).whole_days() as f64;
        return days / denom;
    }

    // Days from start to 31-Dec of start year (inclusive of start, exclusive of next year 1-Jan).
    let start_year_end = Date::from_calendar_date(start.year() + 1, Month::January, 1).unwrap();
    let days_start_year = (start_year_end - start).whole_days() as f64;
    let mut frac = days_start_year / days_in_year(start.year()) as f64;

    // Full intermediate years
    for _year in (start.year() + 1)..end.year() {
        frac += 1.0; // each full year counts as exactly 1.0
    }

    // Days from 1-Jan of end year to end date
    let start_of_end_year = Date::from_calendar_date(end.year(), Month::January, 1).unwrap();
    let days_end_year = (end - start_of_end_year).whole_days() as f64;
    frac += days_end_year / days_in_year(end.year()) as f64;

    frac
}

// -------------------------------------------------------------------------------------------------
// ACT/365L helper
// -------------------------------------------------------------------------------------------------
/// Calculate year fraction for Act/365L convention.
/// 
/// Act/365L uses 366 as denominator if February 29 falls between start (exclusive) and end (inclusive),
/// otherwise uses 365.
fn year_fraction_act_365l(start: Date, end: Date) -> f64 {
    if start == end {
        return 0.0;
    }

    let actual_days = (end - start).whole_days() as f64;
    
    // Check if Feb 29 falls between start (exclusive) and end (inclusive)
    let denominator = if contains_feb_29(start, end) {
        366.0
    } else {
        365.0
    };
    
    actual_days / denominator
}

/// Check if February 29 falls between start (exclusive) and end (inclusive).
fn contains_feb_29(start: Date, end: Date) -> bool {
    let start_year = start.year();
    let end_year = end.year();
    
    // Check each year in the range for Feb 29
    for year in start_year..=end_year {
        if crate::dates::utils::is_leap_year(year) {
            // Try to create Feb 29 for this year
            if let Ok(feb_29) = Date::from_calendar_date(year, Month::February, 29) {
                // Check if Feb 29 is in the interval (start, end]
                if feb_29 > start && feb_29 <= end {
                    return true;
                }
            }
        }
    }
    false
}

// -------------------------------------------------------------------------------------------------
// Bus/252 helper
// -------------------------------------------------------------------------------------------------
/// Count business days between start (inclusive) and end (exclusive) using the given calendar.
fn count_business_days<C: HolidayCalendar + ?Sized>(
    start: Date,
    end: Date,
    calendar: &C,
) -> i32 {
    let mut count = 0;
    let mut current = start;
    
    while current < end {
        if calendar.is_business_day(current) {
            count += 1;
        }
        current += Duration::days(1);
    }
    
    count
}

#[inline]
const fn days_in_year(year: i32) -> i32 {
    if crate::dates::utils::is_leap_year(year) {
        366
    } else {
        365
    }
}



// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn make_date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
    }

    #[test]
    fn act360_basic() {
        let start = make_date(2025, 1, 1);
        let end = start + Duration::days(360);
        let yf = DayCount::Act360.year_fraction(start, end).unwrap();
        assert!((yf - 1.0).abs() < 1e-9);
    }

    #[test]
    fn act365f_year_fraction() {
        let start = make_date(2025, 3, 1);
        let end = make_date(2026, 3, 1);
        let yf = DayCount::Act365F.year_fraction(start, end).unwrap();
        // ACT/365F uses actual days / 365. For 2025-03-01 -> 2026-03-01 there are
        // 365 actual days (no leap day), so expected = 365 / 365.
        let expected = (end - start).whole_days() as f64 / 365.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn thirty_360_end_of_month() {
        let start = make_date(2025, 1, 31);
        let end = make_date(2025, 2, 28);
        let days = DayCount::Thirty360.days(start, end).unwrap();
        assert_eq!(days, 28);
        let yf = DayCount::Thirty360.year_fraction(start, end).unwrap();
        assert!((yf * 360.0 - days as f64).abs() < 1e-9);
    }

    #[test]
    fn actact_spanning_years() {
        let start = make_date(2024, 7, 1); // includes leap year 2024
        let end = make_date(2026, 1, 1);
        let yf = DayCount::ActAct.year_fraction(start, end).unwrap();
        // compute expected manually: part of 2024 (184 days from Jul1 to Jan1 2025), 2025 full year (365 days), part of 2026 (0). Actually end Jan1 so 0.
        let expected = 184.0 / 366.0 + 1.0; // plus full 2025 year 365/365 =1
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn error_on_inverted_dates() {
        let start = make_date(2025, 1, 1);
        let end = make_date(2024, 1, 1);
        assert!(DayCount::Act360.year_fraction(start, end).is_err());
    }

    #[test]
    fn act365l_without_leap_day() {
        // Period that doesn't contain Feb 29
        let start = make_date(2025, 3, 1); // 2025 is not a leap year
        let end = make_date(2025, 9, 1);
        let yf = DayCount::Act365L.year_fraction(start, end).unwrap();
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 365.0; // Should use 365 denominator
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_with_leap_day() {
        // Period that contains Feb 29, 2024 (leap year)
        let start = make_date(2024, 2, 28); // Feb 28, 2024
        let end = make_date(2024, 3, 2);    // Mar 2, 2024 (contains Feb 29)
        let yf = DayCount::Act365L.year_fraction(start, end).unwrap();
        let actual_days = (end - start).whole_days() as f64; // 3 days
        let expected = actual_days / 366.0; // Should use 366 denominator due to Feb 29
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_leap_year_boundary() {
        // Start in leap year, end after leap year
        let start = make_date(2024, 2, 20); // Before Feb 29
        let end = make_date(2025, 1, 15);   // After leap year
        let yf = DayCount::Act365L.year_fraction(start, end).unwrap();
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 366.0; // Should use 366 due to Feb 29 in period
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_leap_year_before_period() {
        // Feb 29 exists in year but falls before start date
        let start = make_date(2024, 3, 1);  // After Feb 29, 2024
        let end = make_date(2024, 6, 1);    // Later in same leap year
        let yf = DayCount::Act365L.year_fraction(start, end).unwrap();
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 365.0; // Should use 365 since Feb 29 not in (start, end]
        assert!((yf - expected).abs() < 1e-9);
    }

    // Simple test-only calendar that treats only weekends as holidays
    #[derive(Debug, Clone, Copy)]
    struct WeekendsOnly;
    
    impl crate::dates::calendar::HolidayCalendar for WeekendsOnly {
        fn is_holiday(&self, _date: Date) -> bool {
            // Return false for all dates; business day logic will still exclude weekends
            false
        }
    }

    #[test]
    fn bus252_with_calendar() {
        // Simple test with weekends-only calendar (Monday to Friday)
        let calendar = WeekendsOnly;
        let start = make_date(2025, 1, 6);  // Monday
        let end = make_date(2025, 1, 13);   // Next Monday (7 calendar days, 5 business days)
        
        let biz_days = DayCount::Bus252.business_days(start, end, &calendar).unwrap();
        assert_eq!(biz_days, 5); // Mon, Tue, Wed, Thu, Fri
        
        let yf = DayCount::Bus252.year_fraction_with_calendar(start, end, &calendar).unwrap();
        let expected = 5.0 / 252.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn bus252_with_nyse_calendar() {
        use crate::dates::holiday::calendars::Nyse;
        
        // Test with a real calendar that has holidays
        let calendar = Nyse;
        let start = make_date(2025, 1, 2);  // Thu (after New Year holiday)
        let end = make_date(2025, 1, 6);    // Mon (4 calendar days)
        
        let biz_days = DayCount::Bus252.business_days(start, end, &calendar).unwrap();
        // Should count Thu, Fri (Sat, Sun are weekends)
        assert_eq!(biz_days, 2);
        
        let yf = DayCount::Bus252.year_fraction_with_calendar(start, end, &calendar).unwrap();
        let expected = 2.0 / 252.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn bus252_error_without_calendar() {
        // Bus/252 should error when using regular methods without calendar
        let start = make_date(2025, 1, 1);
        let end = make_date(2025, 1, 8);
        
        assert!(DayCount::Bus252.days(start, end).is_err());
        assert!(DayCount::Bus252.year_fraction(start, end).is_err());
    }

    #[test]
    fn bus252_equal_dates() {
        let calendar = WeekendsOnly;
        let date = make_date(2025, 1, 1);
        
        let biz_days = DayCount::Bus252.business_days(date, date, &calendar).unwrap();
        assert_eq!(biz_days, 0);
        
        let yf = DayCount::Bus252.year_fraction_with_calendar(date, date, &calendar).unwrap();
        assert_eq!(yf, 0.0);
    }
}
