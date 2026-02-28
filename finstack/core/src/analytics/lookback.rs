//! Lookback period selectors: MTD, QTD, YTD, FYTD.
//!
//! Each function returns a `Range<usize>` into the dates/returns arrays rather
//! than sliced data, so callers slice their own arrays.
//!
//! Delegates to `dates::DateExt` for calendar math.

use crate::dates::{Date, DateExt, FiscalConfig};
use core::ops::Range;

/// Index of the first date on or after `target` via binary search.
fn lower_bound(dates: &[Date], target: Date) -> usize {
    dates.partition_point(|&d| d < target)
}

/// Month-to-date: from the first of the current month through `ref_date`.
///
/// `offset_days` shifts the window start backward.
pub fn mtd_select(dates: &[Date], ref_date: Date, offset_days: i64) -> Range<usize> {
    let month_start = ref_date.end_of_month();
    let month_start = month_start.replace_day(1).unwrap_or(month_start);
    let adj_start = month_start - time::Duration::days(offset_days);
    let lo = lower_bound(dates, adj_start);
    let hi = lower_bound(dates, ref_date + time::Duration::days(1));
    lo..hi
}

/// Quarter-to-date: from the first of the current quarter through `ref_date`.
pub fn qtd_select(dates: &[Date], ref_date: Date, offset_days: i64) -> Range<usize> {
    let q = ref_date.quarter();
    let quarter_start_month = (q - 1) * 3 + 1;
    let (year, _month, _day) = ref_date.to_calendar_date();
    let qtr_start = crate::dates::create_date(
        year,
        time::Month::try_from(quarter_start_month).unwrap_or(time::Month::January),
        1,
    )
    .unwrap_or(ref_date);
    let adj_start = qtr_start - time::Duration::days(offset_days);
    let lo = lower_bound(dates, adj_start);
    let hi = lower_bound(dates, ref_date + time::Duration::days(1));
    lo..hi
}

/// Year-to-date: from January 1 of the current calendar year through `ref_date`.
pub fn ytd_select(dates: &[Date], ref_date: Date, offset_days: i64) -> Range<usize> {
    let (year, _month, _day) = ref_date.to_calendar_date();
    let year_start = crate::dates::create_date(year, time::Month::January, 1).unwrap_or(ref_date);
    let adj_start = year_start - time::Duration::days(offset_days);
    let lo = lower_bound(dates, adj_start);
    let hi = lower_bound(dates, ref_date + time::Duration::days(1));
    lo..hi
}

/// Fiscal-year-to-date: from the start of the fiscal year through `ref_date`.
///
/// Uses `DateExt::fiscal_year(config)` to determine the fiscal year boundary.
pub fn fytd_select(
    dates: &[Date],
    ref_date: Date,
    fiscal_config: FiscalConfig,
    offset_days: i64,
) -> Range<usize> {
    let fy = ref_date.fiscal_year(fiscal_config);
    let fy_start_month =
        time::Month::try_from(fiscal_config.start_month).unwrap_or(time::Month::January);
    let calendar_year = if fiscal_config.start_month == 1 {
        fy
    } else {
        fy - 1
    };
    let fy_start =
        crate::dates::create_date(calendar_year, fy_start_month, fiscal_config.start_day)
            .unwrap_or(ref_date);
    let adj_start = fy_start - time::Duration::days(offset_days);
    let lo = lower_bound(dates, adj_start);
    let hi = lower_bound(dates, ref_date + time::Duration::days(1));
    lo..hi
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        crate::dates::create_date(y, Month::try_from(m).expect("valid month"), day)
            .expect("valid date")
    }

    fn daily_dates(start: Date, n: usize) -> Vec<Date> {
        (0..n)
            .map(|i| start + time::Duration::days(i as i64))
            .collect()
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
}
