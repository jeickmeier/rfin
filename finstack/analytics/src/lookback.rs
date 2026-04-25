//! Lookback period selectors: MTD, QTD, YTD, FYTD.
//!
//! Each function returns a `Range<usize>` into the dates/returns arrays rather
//! than sliced data, so callers slice their own arrays.
//!
//! Delegates to `dates::DateExt` for calendar math.

use crate::dates::{Date, DateExt, Duration, FiscalConfig, Month};
use core::ops::Range;

/// Index of the first date on or after `target` via binary search.
fn lower_bound(dates: &[Date], target: Date) -> usize {
    dates.partition_point(|&d| d < target)
}

/// Shared range builder: `[period_start - offset_days, ref_date]` inclusive.
fn select_range(dates: &[Date], period_start: Date, ref_date: Date, offset_days: i64) -> Range<usize> {
    let adj_start = period_start - Duration::days(offset_days);
    let lo = lower_bound(dates, adj_start);
    let hi = lower_bound(dates, ref_date + Duration::days(1));
    lo..hi
}

/// Month-to-date index range: from the first calendar day of `ref_date`'s
/// month through `ref_date` (inclusive).
///
/// `offset_days` shifts the window start backward by that many days,
/// which is useful when the first trading day of the month does not fall
/// on the 1st.
///
/// # Arguments
///
/// * `dates`       - Sorted slice of observation dates.
/// * `ref_date`    - Reference date (typically "today").
/// * `offset_days` - Number of days to subtract from the computed start date.
///
/// # Returns
///
/// A `Range<usize>` into `dates` covering the MTD window.
/// The range may be empty if no dates fall within the window.
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{Date, Month};
/// use finstack_analytics::lookback::mtd_select;
///
/// let dates: Vec<Date> = (1..=28)
///     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
///     .collect();
/// let range = mtd_select(&dates, Date::from_calendar_date(2025, Month::January, 15).unwrap(), 0);
/// assert_eq!(range.start, 0);
/// assert_eq!(range.end, 15);
/// ```
pub fn mtd_select(dates: &[Date], ref_date: Date, offset_days: i64) -> Range<usize> {
    let month_start = ref_date.end_of_month();
    let month_start = month_start.replace_day(1).unwrap_or(month_start);
    select_range(dates, month_start, ref_date, offset_days)
}

/// Quarter-to-date index range: from the first calendar day of `ref_date`'s
/// quarter through `ref_date` (inclusive).
///
/// Quarter boundaries follow calendar convention: Q1 = Jan–Mar,
/// Q2 = Apr–Jun, Q3 = Jul–Sep, Q4 = Oct–Dec.
///
/// # Arguments
///
/// * `dates`       - Sorted slice of observation dates.
/// * `ref_date`    - Reference date (typically "today").
/// * `offset_days` - Number of days to subtract from the computed start date.
///
/// # Returns
///
/// A `Range<usize>` into `dates` covering the QTD window.
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{Date, Duration, Month};
/// use finstack_analytics::lookback::qtd_select;
///
/// let dates: Vec<Date> = (1..=60)
///     .map(|d| Date::from_calendar_date(2025, Month::January, 1).unwrap()
///         + Duration::days(d - 1))
///     .collect();
/// let range = qtd_select(&dates, Date::from_calendar_date(2025, Month::February, 15).unwrap(), 0);
/// // Q1 starts Jan 1 → range should include all dates up through Feb 15.
/// assert_eq!(range.start, 0);
/// assert!(range.end > 30);
/// ```
pub fn qtd_select(dates: &[Date], ref_date: Date, offset_days: i64) -> Range<usize> {
    let q = ref_date.quarter();
    let quarter_start_month = (q - 1) * 3 + 1;
    let (year, _month, _day) = ref_date.to_calendar_date();
    let qtr_start = crate::dates::create_date(
        year,
        Month::try_from(quarter_start_month).unwrap_or(Month::January),
        1,
    )
    .unwrap_or(ref_date);
    select_range(dates, qtr_start, ref_date, offset_days)
}

/// Year-to-date index range: from January 1 of `ref_date`'s calendar year
/// through `ref_date` (inclusive).
///
/// # Arguments
///
/// * `dates`       - Sorted slice of observation dates.
/// * `ref_date`    - Reference date (typically "today").
/// * `offset_days` - Number of days to subtract from January 1.
///
/// # Returns
///
/// A `Range<usize>` into `dates` covering the YTD window.
///
/// # Examples
///
/// ```rust
/// use finstack_core::dates::{Date, Duration, Month};
/// use finstack_analytics::lookback::ytd_select;
///
/// let dates: Vec<Date> = (0..60)
///     .map(|d| Date::from_calendar_date(2025, Month::January, 1).unwrap()
///         + Duration::days(d))
///     .collect();
/// let range = ytd_select(&dates, Date::from_calendar_date(2025, Month::February, 15).unwrap(), 0);
/// assert_eq!(range.start, 0);
/// assert!(range.end > 30);
/// ```
pub fn ytd_select(dates: &[Date], ref_date: Date, offset_days: i64) -> Range<usize> {
    let (year, _month, _day) = ref_date.to_calendar_date();
    let year_start = crate::dates::create_date(year, Month::January, 1).unwrap_or(ref_date);
    select_range(dates, year_start, ref_date, offset_days)
}

/// Fiscal-year-to-date index range: from the start of the fiscal year
/// containing `ref_date` through `ref_date` (inclusive).
///
/// The fiscal year start is determined by [`FiscalConfig`] (start month and
/// day). For example, the US federal fiscal year starts October 1.
///
/// # Arguments
///
/// * `dates`        - Sorted slice of observation dates.
/// * `ref_date`     - Reference date (typically "today").
/// * `fiscal_config`- Fiscal year configuration (start month, start day).
/// * `offset_days`  - Number of days to subtract from the fiscal year start.
///
/// # Returns
///
/// A `Range<usize>` into `dates` covering the FYTD window.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::lookback::fytd_select;
/// use finstack_core::dates::{Date, Duration, FiscalConfig, Month};
///
/// // US federal fiscal year: Oct 1 → Sep 30.
/// let dates: Vec<Date> = (0..120)
///     .map(|d| Date::from_calendar_date(2024, Month::October, 1).unwrap()
///         + Duration::days(d))
///     .collect();
/// let config = FiscalConfig::us_federal();
/// let range = fytd_select(
///     &dates,
///     Date::from_calendar_date(2025, Month::January, 15).unwrap(),
///     config,
///     0,
/// );
/// assert_eq!(range.start, 0);
/// assert!(range.end > 0);
/// ```
pub fn fytd_select(
    dates: &[Date],
    ref_date: Date,
    fiscal_config: FiscalConfig,
    offset_days: i64,
) -> Range<usize> {
    let fy = ref_date.fiscal_year(fiscal_config);
    let fy_start_month = Month::try_from(fiscal_config.start_month).unwrap_or(Month::January);
    let calendar_year = if fiscal_config.start_month == 1 && fiscal_config.start_day <= 1 {
        fy
    } else {
        fy - 1
    };
    let fy_start =
        crate::dates::create_date(calendar_year, fy_start_month, fiscal_config.start_day)
            .unwrap_or(ref_date);
    select_range(dates, fy_start, ref_date, offset_days)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(y: i32, m: u8, day: u8) -> Date {
        crate::dates::create_date(y, Month::try_from(m).expect("valid month"), day)
            .expect("valid date")
    }

    fn daily_dates(start: Date, n: usize) -> Vec<Date> {
        (0..n).map(|i| start + Duration::days(i as i64)).collect()
    }

    #[test]
    fn ytd_select_basic() {
        let dates = daily_dates(d(2025, 1, 1), 60);
        let range = ytd_select(&dates, d(2025, 2, 15), 0);
        assert_eq!(range.start, 0);
        assert!(range.end > 30);
    }

    #[test]
    fn mtd_select_basic() {
        let dates = daily_dates(d(2025, 1, 1), 60);
        let range = mtd_select(&dates, d(2025, 2, 15), 0);
        assert!(range.start > 0);
    }

    #[test]
    fn qtd_select_q1() {
        let dates = daily_dates(d(2025, 1, 1), 90);
        let range = qtd_select(&dates, d(2025, 3, 15), 0);
        assert_eq!(range.start, 0);
    }

    #[test]
    fn fytd_select_us_federal() {
        let dates = daily_dates(d(2024, 10, 1), 120);
        let config = FiscalConfig::us_federal();
        let range = fytd_select(&dates, d(2025, 1, 15), config, 0);
        assert_eq!(range.start, 0);
    }

    #[test]
    fn fytd_select_january_mid_month_start_uses_prior_calendar_year() {
        let dates = daily_dates(d(2025, 1, 1), 40);
        let config = FiscalConfig::new(1, 15).expect("valid fiscal config");
        let range = fytd_select(&dates, d(2025, 1, 20), config, 0);
        assert_eq!(range.start, 14);
        assert_eq!(range.end, 20);
    }
}
