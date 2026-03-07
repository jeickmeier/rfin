//! Date and schedule generation utilities.
//!
//! This module provides functions for generating payment date schedules based on
//! frequency, stub rules, and business day conventions.
//!
//! ## Responsibilities
//!
//! - Generate period schedules with `build_dates` (strict validation)
//! - Create `PeriodSchedule` with helper maps for previous date lookups
//! - Apply business day adjustments using calendars

use finstack_core::dates::{
    BusinessDayConvention, Date, DateExt, ScheduleBuilder, StubKind, Tenor,
};

use super::calendar::resolve_calendar_strict;

/// Accrual period with payment timing.
///
/// This is the canonical period type used across the cashflow builder (schedule
/// compilation) and rates instruments (enriched with reset dates and year
/// fractions). Fields that are not relevant in a given context are left at
/// their defaults (`None` / `0.0`).
#[derive(Debug, Clone, Copy)]
pub struct SchedulePeriod {
    /// Accrual start date (inclusive).
    pub accrual_start: Date,
    /// Accrual end date (exclusive boundary).
    pub accrual_end: Date,
    /// Payment date after applying payment lag.
    pub payment_date: Date,
    /// Optional reset/fixing date for floating legs.
    pub reset_date: Option<Date>,
    /// Accrual year fraction for the period (0.0 when not computed).
    pub accrual_year_fraction: f64,
}

/// Period schedule output with business day adjustment tracking.
#[derive(Debug, Clone)]
pub struct PeriodSchedule {
    /// Generated accrual/payment periods.
    pub periods: Vec<SchedulePeriod>,
    /// Payment dates for each period (for backward-compatible callers).
    pub dates: Vec<Date>,
    /// Set of payment dates that correspond to first or last periods.
    pub first_or_last: finstack_core::HashSet<Date>,
}

/// Build a schedule between start/end with strict error handling.
///
/// # Errors
///
/// Returns `finstack_core::Error` when:
/// - `calendar_id` is provided but calendar is not found
/// - Schedule generation fails due to invalid date ranges
/// - Business day adjustment fails
///
/// # Example
///
/// ```rust
/// use finstack_core::dates::{Date, Tenor, BusinessDayConvention, StubKind, create_date};
/// use finstack_valuations::cashflow::builder::date_generation::build_dates;
/// use time::Month;
///
/// let start = create_date(2025, Month::January, 15)?;
/// let end = create_date(2025, Month::July, 15)?;
/// let sched = build_dates(
///     start,
///     end,
///     Tenor::quarterly(),
///     StubKind::None,
///     BusinessDayConvention::Following,
///     false,
///     0,
///     "weekends_only",
/// )?;
/// assert!(sched.dates.len() >= 2);
/// # Ok::<(), finstack_core::Error>(())
/// ```
#[allow(clippy::too_many_arguments)]
pub fn build_dates(
    start: Date,
    end: Date,
    freq: Tenor,
    stub: StubKind,
    bdc: BusinessDayConvention,
    end_of_month: bool,
    payment_lag_days: i32,
    calendar_id: &str,
) -> finstack_core::Result<PeriodSchedule> {
    let mut builder = ScheduleBuilder::new(start, end)?
        .frequency(freq)
        .stub_rule(stub)
        .end_of_month(end_of_month);
    let cal = resolve_calendar_strict(calendar_id)?;
    builder = builder.adjust_with(bdc, cal);

    let schedule = builder.build()?;
    let mut dates = schedule.dates;

    // Ensure the schedule reaches the requested `end` when it falls exactly on a regular boundary.
    //
    // Some schedules can terminate at the penultimate boundary (e.g. 5Y semi-annual coupons)
    // which would omit the maturity payment date from downstream coupon emission.
    if matches!(dates.last(), Some(last) if *last < end) {
        dates.push(end);
    }

    if dates.len() < 2 {
        return Ok(PeriodSchedule {
            periods: Vec::new(),
            dates: Vec::new(),
            first_or_last: finstack_core::HashSet::default(),
        });
    }

    let mut periods = Vec::with_capacity(dates.len().saturating_sub(1));
    let mut payment_dates = Vec::with_capacity(dates.len().saturating_sub(1));
    let mut first_or_last = finstack_core::HashSet::default();

    for window in dates.windows(2) {
        let accrual_start = window[0];
        let accrual_end = window[1];
        let payment_date = if payment_lag_days == 0 {
            accrual_end
        } else {
            accrual_end.add_business_days(payment_lag_days, cal)?
        };
        periods.push(SchedulePeriod {
            accrual_start,
            accrual_end,
            payment_date,
            reset_date: None,
            accrual_year_fraction: 0.0,
        });
        payment_dates.push(payment_date);
    }

    if let Some(first) = payment_dates.first() {
        first_or_last.insert(*first);
    }
    if let Some(last) = payment_dates.last() {
        first_or_last.insert(*last);
    }

    Ok(PeriodSchedule {
        periods,
        dates: payment_dates,
        first_or_last,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("Valid month (1-12)"), day)
            .expect("Valid test date")
    }

    #[test]
    fn build_dates_errors_on_unknown_calendar() {
        let res = build_dates(
            d(2025, 1, 1),
            d(2025, 4, 1),
            Tenor::quarterly(),
            StubKind::None,
            BusinessDayConvention::Following,
            false,
            0,
            "NOT_A_CAL",
        );
        assert!(res.is_err());
    }
}
