//! Period schedule utilities for rates pricing.
//!
//! Provides canonical accrual/payment periods built on top of the cashflow
//! builder's date generation and calendar policy.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, DayCountCtx, StubKind, Tenor};

use super::calendar::resolve_calendar_strict;
use super::date_generation::build_dates;
use super::emission::compute_reset_date;

/// A standardized accrual/payment period for rates instruments.
#[derive(Debug, Clone)]
pub struct SchedulePeriod {
    /// Start of the accrual period.
    pub accrual_start: Date,
    /// End of the accrual period (schedule boundary).
    pub accrual_end: Date,
    /// Payment date after applying payment lag.
    pub payment_date: Date,
    /// Optional reset/fixing date for floating legs.
    pub reset_date: Option<Date>,
    /// Accrual year fraction for the period.
    pub accrual_year_fraction: f64,
}

/// Parameters for building schedule periods.
pub struct BuildPeriodsParams<'a> {
    /// Start date of the schedule.
    pub start: Date,
    /// End date (maturity) of the schedule.
    pub end: Date,
    /// Coupon/payment frequency.
    pub frequency: Tenor,
    /// Stub handling convention.
    pub stub: StubKind,
    /// Business day convention for date adjustments.
    pub bdc: BusinessDayConvention,
    /// Holiday calendar identifier (use "weekends_only" for weekends-only adjustments).
    pub calendar_id: &'a str,
    /// Whether to enforce end-of-month rolling.
    pub end_of_month: bool,
    /// Day count convention for accrual fractions.
    pub day_count: DayCount,
    /// Payment lag in business days after accrual end.
    pub payment_lag_days: i32,
    /// Optional reset lag in business days before accrual start.
    pub reset_lag_days: Option<i32>,
}

/// Build canonical schedule periods with consistent market conventions.
pub fn build_periods(params: BuildPeriodsParams<'_>) -> finstack_core::Result<Vec<SchedulePeriod>> {
    let cal = resolve_calendar_strict(params.calendar_id)?;
    let sched = build_dates(
        params.start,
        params.end,
        params.frequency,
        params.stub,
        params.bdc,
        params.end_of_month,
        params.payment_lag_days,
        params.calendar_id,
    )?;

    if sched.periods.is_empty() {
        return Ok(Vec::new());
    }

    let dc_ctx = DayCountCtx {
        calendar: Some(cal),
        frequency: Some(params.frequency),
        bus_basis: None,
    };
    let mut periods = Vec::with_capacity(sched.periods.len());
    for period in sched.periods {
        let accrual_year_fraction =
            params
                .day_count
                .year_fraction(period.accrual_start, period.accrual_end, dc_ctx)?;

        let reset_date = params
            .reset_lag_days
            .map(|lag| {
                compute_reset_date(period.accrual_start, lag, params.bdc, params.calendar_id)
            })
            .transpose()?;

        periods.push(SchedulePeriod {
            accrual_start: period.accrual_start,
            accrual_end: period.accrual_end,
            payment_date: period.payment_date,
            reset_date,
            accrual_year_fraction,
        });
    }

    Ok(periods)
}
