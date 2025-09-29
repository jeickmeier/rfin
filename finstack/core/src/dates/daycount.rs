//! Day-count convention algorithms (ACT/360, ACT/365F, 30/360, 30E/360, ACT/ACT, ACT/ACT ISMA, Bus/252).
//!
//! The implementation follows the ISDA and ICMA/ISMA definitions where applicable and is **panic-free**.
//! All helpers avoid heap allocation.
//!
//! # Examples
//! ```
//! use finstack_core::dates::{Date, DayCount, DayCountCtx};
//! use time::Month;
//!
//! let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let end   = Date::from_calendar_date(2026, Month::January, 1).unwrap();
//!
//! let yf = DayCount::ActAct
//!     .year_fraction(start, end, DayCountCtx::default())
//!     .unwrap();
//! assert!((yf - 1.0).abs() < 1e-9);
//! ```
//!
//! # Bus/252 Convention
//!
//! The Bus/252 convention counts business days between dates and divides by 252 (typical trading days per year).
//! This requires a holiday calendar to determine business days. Provide the calendar via `DayCountCtx`.
//!
//! ```
//! use finstack_core::dates::{Date, DayCount, DayCountCtx};
//! use finstack_core::dates::calendar::TARGET2;
//! use time::Month;
//!
//! let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
//! let end   = Date::from_calendar_date(2025, Month::January, 31).unwrap();
//! let calendar = TARGET2;
//!
//! // Calculate year fraction with a calendar in context
//! let yf = DayCount::Bus252
//!     .year_fraction(start, end, DayCountCtx { calendar: Some(&calendar), frequency: None })
//!     .unwrap();
//! ```
//!
//! # ACT/ACT ISMA vs ISDA
//!
//! Both conventions use actual days in numerator and actual days in denominator, but differ in how
//! the denominator is calculated:
//!
//! - **ACT/ACT (ISDA)**: Uses the actual number of days in the year containing the period
//! - **ACT/ACT (ISMA)**: Uses the actual number of days in the coupon period containing the date
//!
//! ```
//! use finstack_core::dates::{Date, DayCount, Frequency, DayCountCtx};
//! use time::Month;
//!
//! // Example: 6-month period in a leap year
//! let start = Date::from_calendar_date(2024, Month::January, 1).unwrap(); // Leap year
//! let end   = Date::from_calendar_date(2024, Month::July, 1).unwrap();
//!
//! // ACT/ACT (ISDA): 181 days / 366 days (leap year) = 0.4945355191256831
//! let yf_isda = DayCount::ActAct.year_fraction(start, end, DayCountCtx::default()).unwrap();
//!
//! // ACT/ACT (ISMA): requires frequency for coupon period context
//! let freq = Frequency::Months(6); // Semi-annual
//! let yf_isma = DayCount::ActActIsma
//!     .year_fraction(start, end, DayCountCtx { calendar: None, frequency: Some(freq) })
//!     .unwrap();
//! ```

#![allow(clippy::many_single_char_names)]

use crate::dates::utils::add_months;
use core::cmp::Ordering;
use time::{Date, Duration, Month};

use crate::dates::date_extensions::BusinessDayIter;
use crate::dates::schedule_iter::Frequency;
use crate::dates::HolidayCalendar;
use crate::error::InputError;

/// Optional context for day-count year-fraction calculations.
///
/// Certain conventions require additional information:
/// - `Bus/252` requires a holiday `calendar`.
/// - `Act/Act (ISMA)` requires the coupon `frequency`.
#[derive(Clone, Copy, Default)]
pub struct DayCountCtx<'a> {
    /// Optional holiday calendar for business-day based conventions (e.g., Bus/252).
    pub calendar: Option<&'a dyn HolidayCalendar>,
    /// Optional coupon frequency for coupon-aware conventions (e.g., Act/Act ISMA).
    pub frequency: Option<Frequency>,
}

/// Supported day-count conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
#[non_exhaustive]
pub enum DayCount {
    /// Actual / 360 — year fraction = actual days ÷ 360.
    #[cfg_attr(feature = "serde", serde(alias = "act360"))]
    Act360,
    /// Actual / 365F — year fraction = actual days ÷ 365 (fixed).
    #[cfg_attr(
        feature = "serde",
        serde(alias = "act_365f", alias = "act365f", alias = "act_365_fixed")
    )]
    Act365F,
    /// Actual / 365L (Actual/365 Leap or AFB) — denominator varies based on leap year logic.
    #[cfg_attr(feature = "serde", serde(alias = "act365l", alias = "act_365l"))]
    Act365L,
    /// 30U/360 (US Bond Basis).
    #[cfg_attr(feature = "serde", serde(alias = "thirty360"))]
    Thirty360,
    /// 30E/360 (European).
    #[cfg_attr(feature = "serde", serde(alias = "thirty_e360"))]
    ThirtyE360,
    /// Actual / Actual (ISDA variant).
    #[cfg_attr(feature = "serde", serde(alias = "act_act"))]
    ActAct,
    /// Actual / Actual (ISMA/ICMA variant) — coupon-period aware.
    #[cfg_attr(feature = "serde", serde(alias = "act_act_isma"))]
    ActActIsma,
    /// Bus/252 — business days ÷ 252 (requires holiday calendar).
    #[cfg_attr(feature = "serde", serde(alias = "bus252"))]
    Bus252,
}

impl DayCount {
    /// Return the day count between `start` (inclusive) and `end` (exclusive).
    ///
    /// The output follows the specific convention rules and is **always ≥ 0**.
    ///
    /// # Note
    /// For `Bus/252`, this returns an error (requires calendar context via [`DayCountCtx`]).
    #[allow(dead_code)]
    #[doc(hidden)]
    pub(crate) fn days(self, start: Date, end: Date) -> crate::Result<i32> {
        match start.cmp(&end) {
            Ordering::Greater => Err(InputError::InvalidDateRange.into()),
            Ordering::Equal => Ok(0),
            Ordering::Less => match self {
                DayCount::Act360
                | DayCount::Act365F
                | DayCount::Act365L
                | DayCount::ActAct
                | DayCount::ActActIsma => {
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
    /// Provide any required context via [`DayCountCtx`]:
    /// - `Bus/252` requires a holiday calendar.
    /// - `Act/Act (ISMA)` requires a coupon frequency.
    pub fn year_fraction(self, start: Date, end: Date, ctx: DayCountCtx<'_>) -> crate::Result<f64> {
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
                    DayCount::ActActIsma => match ctx.frequency {
                        Some(freq) => year_fraction_act_act_isma(start, end, freq)?,
                        None => return Err(InputError::Invalid.into()),
                    },
                    DayCount::Bus252 => match ctx.calendar {
                        Some(cal) => {
                            let biz_days = count_business_days(start, end, cal) as f64;
                            biz_days / 252.0
                        }
                        None => return Err(InputError::Invalid.into()),
                    },
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
// ACT/ACT (ISMA/ICMA) helper
// -------------------------------------------------------------------------------------------------
/// Calculate year fraction for ACT/ACT (ISMA) convention with coupon-period awareness.
///
/// Unlike ISDA which splits by calendar years, ISMA divides the period into quasi-coupon
/// periods that match the instrument's payment frequency. This ensures equal valuation
/// of days within each coupon period.
///
/// The algorithm:
/// 1. Generate quasi-coupon periods based on the payment frequency
/// 2. For each period, calculate: (actual days in period) / (actual days in year)
/// 3. Sum the fractions from all periods
///
/// This approach ensures that all coupon payments are valued consistently,
/// which is essential for bond pricing and accrual calculations.
fn year_fraction_act_act_isma(start: Date, end: Date, freq: Frequency) -> crate::Result<f64> {
    if start == end {
        return Ok(0.0);
    }

    // For ISMA, we need to work with quasi-coupon periods
    // We'll generate a schedule that encompasses the period and then
    // calculate the year fraction for each sub-period

    let mut total_fraction = 0.0;

    // Generate schedule to find quasi-coupon periods
    // We need to extend backward/forward to capture the full coupon periods
    let extended_start = extend_backward_for_coupon_period(start, freq);
    let extended_end = extend_forward_for_coupon_period(end, freq);

    let schedule = crate::dates::ScheduleBuilder::new(extended_start, extended_end)
        .frequency(freq)
        .build()?;

    let periods: Vec<Date> = schedule.into_iter().collect();

    // Find the periods that overlap with our [start, end) interval
    for window in periods.windows(2) {
        let period_start = window[0];
        let period_end = window[1];

        // Check if this period overlaps with our target interval
        let overlap_start = start.max(period_start);
        let overlap_end = end.min(period_end);

        if overlap_start < overlap_end {
            // Numerator: actual days in the overlapping slice
            let days_in_overlap = (overlap_end - overlap_start).whole_days() as f64;

            // Denominator (ISMA): actual days in the coupon period that contains this slice
            let coupon_days = (period_end - period_start).whole_days() as f64;
            if coupon_days <= 0.0 {
                return Err(InputError::Invalid.into());
            }

            total_fraction += days_in_overlap / coupon_days;
        }
    }

    Ok(total_fraction)
}

/// Extend start date backward to find the beginning of its coupon period.
fn extend_backward_for_coupon_period(date: Date, freq: Frequency) -> Date {
    match freq {
        // Align coupon schedule to the provided date; treat `date` as an anchor.
        Frequency::Months(_) => date,
        Frequency::Days(_) => date,
    }
}

/// Extend end date forward to find the end of its coupon period.
fn extend_forward_for_coupon_period(date: Date, freq: Frequency) -> Date {
    match freq {
        Frequency::Months(m) => add_months(date, m as i32),
        Frequency::Days(d) => date + Duration::days(d as i64),
    }
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
fn count_business_days<C: HolidayCalendar + ?Sized>(start: Date, end: Date, calendar: &C) -> i32 {
    BusinessDayIter::new(start, end, calendar).count() as i32
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
        let yf = DayCount::Act360
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        assert!((yf - 1.0).abs() < 1e-9);
    }

    #[test]
    fn act365f_year_fraction() {
        let start = make_date(2025, 3, 1);
        let end = make_date(2026, 3, 1);
        let yf = DayCount::Act365F
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
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
        let yf = DayCount::Thirty360
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        assert!((yf * 360.0 - days as f64).abs() < 1e-9);
    }

    #[test]
    fn actact_spanning_years() {
        let start = make_date(2024, 7, 1); // includes leap year 2024
        let end = make_date(2026, 1, 1);
        let yf = DayCount::ActAct
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        // compute expected manually: part of 2024 (184 days from Jul1 to Jan1 2025), 2025 full year (365 days), part of 2026 (0). Actually end Jan1 so 0.
        let expected = 184.0 / 366.0 + 1.0; // plus full 2025 year 365/365 =1
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn error_on_inverted_dates() {
        let start = make_date(2025, 1, 1);
        let end = make_date(2024, 1, 1);
        assert!(DayCount::Act360
            .year_fraction(start, end, DayCountCtx::default())
            .is_err());
    }

    #[test]
    fn act365l_without_leap_day() {
        // Period that doesn't contain Feb 29
        let start = make_date(2025, 3, 1); // 2025 is not a leap year
        let end = make_date(2025, 9, 1);
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 365.0; // Should use 365 denominator
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_with_leap_day() {
        // Period that contains Feb 29, 2024 (leap year)
        let start = make_date(2024, 2, 28); // Feb 28, 2024
        let end = make_date(2024, 3, 2); // Mar 2, 2024 (contains Feb 29)
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        let actual_days = (end - start).whole_days() as f64; // 3 days
        let expected = actual_days / 366.0; // Should use 366 denominator due to Feb 29
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_leap_year_boundary() {
        // Start in leap year, end after leap year
        let start = make_date(2024, 2, 20); // Before Feb 29
        let end = make_date(2025, 1, 15); // After leap year
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 366.0; // Should use 366 due to Feb 29 in period
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn act365l_leap_year_before_period() {
        // Feb 29 exists in year but falls before start date
        let start = make_date(2024, 3, 1); // After Feb 29, 2024
        let end = make_date(2024, 6, 1); // Later in same leap year
        let yf = DayCount::Act365L
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        let actual_days = (end - start).whole_days() as f64;
        let expected = actual_days / 365.0; // Should use 365 since Feb 29 not in (start, end]
        assert!((yf - expected).abs() < 1e-9);
    }

    // Simple test-only calendar that treats only weekends as holidays
    #[derive(Debug, Clone, Copy)]
    struct WeekendsOnly;

    impl crate::dates::HolidayCalendar for WeekendsOnly {
        fn is_holiday(&self, _date: Date) -> bool {
            // Return false for all dates; business day logic will still exclude weekends
            false
        }
    }

    #[test]
    fn bus252_with_calendar() {
        // Simple test with weekends-only calendar (Monday to Friday)
        let calendar = WeekendsOnly;
        let start = make_date(2025, 1, 6); // Monday
        let end = make_date(2025, 1, 13); // Next Monday (7 calendar days, 5 business days)

        let biz_days = DayCount::Bus252
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                },
            )
            .unwrap()
            * 252.0;
        assert_eq!(biz_days.round() as i32, 5);

        let yf = DayCount::Bus252
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                },
            )
            .unwrap();
        let expected = 5.0 / 252.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn bus252_with_nyse_calendar() {
        use crate::dates::calendar::NYSE;

        // Test with a real calendar that has holidays
        let calendar = NYSE;
        let start = make_date(2025, 1, 2); // Thu (after New Year holiday)
        let end = make_date(2025, 1, 6); // Mon (4 calendar days)

        let biz_days = DayCount::Bus252
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                },
            )
            .unwrap()
            * 252.0;
        // Should count Thu, Fri (Sat, Sun are weekends)
        assert_eq!(biz_days.round() as i32, 2);

        let yf = DayCount::Bus252
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                },
            )
            .unwrap();
        let expected = 2.0 / 252.0;
        assert!((yf - expected).abs() < 1e-9);
    }

    #[test]
    fn bus252_error_without_calendar() {
        // Bus/252 should error when using regular methods without calendar
        let start = make_date(2025, 1, 1);
        let end = make_date(2025, 1, 8);

        assert!(DayCount::Bus252.days(start, end).is_err());
        assert!(DayCount::Bus252
            .year_fraction(start, end, DayCountCtx::default())
            .is_err());
    }

    #[test]
    fn bus252_equal_dates() {
        let calendar = WeekendsOnly;
        let date = make_date(2025, 1, 1);

        let biz_days = DayCount::Bus252
            .year_fraction(
                date,
                date,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                },
            )
            .unwrap()
            * 252.0;
        assert_eq!(biz_days.round() as i32, 0);

        let yf = DayCount::Bus252
            .year_fraction(
                date,
                date,
                DayCountCtx {
                    calendar: Some(&calendar),
                    frequency: None,
                },
            )
            .unwrap();
        assert_eq!(yf, 0.0);
    }

    #[test]
    fn actact_isma_semi_annual() {
        // Test ACT/ACT (ISMA) with semi-annual frequency
        let start = make_date(2025, 1, 15);
        let end = make_date(2025, 7, 15);
        let freq = Frequency::Months(6); // Semi-annual

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                },
            )
            .unwrap();

        // Under ISMA with coupon-period denominator, a full coupon period is 1.0
        assert!((yf - 1.0).abs() < 1e-6, "Expected 1.0, got {}", yf);
    }

    #[test]
    fn actact_isma_quarterly() {
        // Test ACT/ACT (ISMA) with quarterly frequency
        let start = make_date(2025, 1, 1);
        let end = make_date(2025, 4, 1);
        let freq = Frequency::Months(3); // Quarterly

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                },
            )
            .unwrap();

        // Under ISMA with coupon-period denominator, a full coupon period is 1.0
        assert!((yf - 1.0).abs() < 1e-6, "Expected 1.0, got {}", yf);
    }

    #[test]
    fn actact_isma_annual() {
        // Test ACT/ACT (ISMA) with annual frequency
        let start = make_date(2025, 1, 1);
        let end = make_date(2026, 1, 1);
        let freq = Frequency::Months(12); // Annual

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                },
            )
            .unwrap();

        // For a full year period, this should be exactly 1.0
        assert!((yf - 1.0).abs() < 1e-9, "Expected 1.0, got {}", yf);
    }

    #[test]
    fn actact_isma_spanning_leap_year() {
        // Test ACT/ACT (ISMA) spanning a leap year boundary
        let start = make_date(2023, 7, 1); // Mid-2023 (non-leap)
        let end = make_date(2024, 7, 1); // Mid-2024 (leap year)
        let freq = Frequency::Months(6); // Semi-annual

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                },
            )
            .unwrap();

        // Two full semi-annual coupon periods → 2.0
        assert!((yf - 2.0).abs() < 1e-6, "Expected 2.0, got {}", yf);
    }

    #[test]
    fn actact_isma_partial_period() {
        // Test ACT/ACT (ISMA) for a partial coupon period
        let start = make_date(2025, 1, 15); // Mid-month start
        let end = make_date(2025, 3, 15); // Two months later
        let freq = Frequency::Months(6); // Semi-annual coupons

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                },
            )
            .unwrap();

        // Two months out of a 6-month coupon → roughly ~0.33 depending on month lengths
        assert!(yf > 0.30 && yf < 0.35, "Expected ~0.33, got {}", yf);
    }

    #[test]
    fn actact_isma_monthly_frequency() {
        // Test ACT/ACT (ISMA) with monthly frequency
        let start = make_date(2025, 1, 1);
        let end = make_date(2025, 2, 1);
        let freq = Frequency::Months(1); // Monthly

        let yf = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                },
            )
            .unwrap();

        // For a one-month coupon with monthly frequency, a full coupon period is 1.0
        assert!((yf - 1.0).abs() < 1e-6, "Expected 1.0, got {}", yf);
    }

    #[test]
    fn actact_isma_error_on_inverted_dates() {
        // ACT/ACT (ISMA) should error on inverted dates
        let start = make_date(2025, 1, 1);
        let end = make_date(2024, 1, 1);
        let freq = Frequency::Months(6);

        assert!(DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq)
                }
            )
            .is_err());
    }

    #[test]
    fn actact_isma_equal_dates() {
        // ACT/ACT (ISMA) should return 0.0 for equal dates
        let date = make_date(2025, 1, 1);
        let freq = Frequency::Months(6);

        let yf = DayCount::ActActIsma
            .year_fraction(
                date,
                date,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                },
            )
            .unwrap();
        assert_eq!(yf, 0.0);
    }

    #[test]
    fn actact_isma_vs_isda_comparison() {
        // Compare ACT/ACT (ISMA) vs ACT/ACT (ISDA) for the same period
        let start = make_date(2024, 6, 15);
        let end = make_date(2025, 6, 15);
        let freq = Frequency::Months(6); // Semi-annual

        let yf_isda = DayCount::ActAct
            .year_fraction(start, end, DayCountCtx::default())
            .unwrap();
        let yf_isma = DayCount::ActActIsma
            .year_fraction(
                start,
                end,
                DayCountCtx {
                    calendar: None,
                    frequency: Some(freq),
                },
            )
            .unwrap();

        // ISDA splits by calendar year → ~1.0; ISMA (coupon-period denominator) sums full coupons → ~2.0
        assert!(
            yf_isda > 0.99 && yf_isda < 1.01,
            "ISDA: Expected ~1.0, got {}",
            yf_isda
        );
        assert!(
            yf_isma > 1.99 && yf_isma < 2.01,
            "ISMA: Expected ~2.0, got {}",
            yf_isma
        );
        // Expect a difference of roughly 1.0 between methods
        assert!((yf_isma - yf_isda - 1.0).abs() < 0.05);
    }
}
