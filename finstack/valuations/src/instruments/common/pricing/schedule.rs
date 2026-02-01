//! Canonical schedule building for rates instruments.
//!
//! Provides a single helper to build accrual/payment periods with consistent
//! business-day adjustment, lag handling, and year-fraction calculation.

use finstack_core::dates::calendar::Calendar;
use finstack_core::dates::CalendarRegistry;
use finstack_core::dates::{
    adjust, BusinessDayConvention, Date, DateExt, DayCount, DayCountCtx, ScheduleBuilder, StubKind,
    Tenor,
};
use finstack_core::Result;

/// Weekends-only calendar for cases where no holiday calendar is supplied.
///
/// Weekends remain non-business days; no holiday rules are applied.
const WEEKENDS_ONLY: Calendar = Calendar::new("weekends_only", "Weekends Only", true, &[]);

/// A standardized accrual/payment period for rates instruments.
#[derive(Clone, Debug)]
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

fn resolve_calendar_strict(
    calendar_id: Option<&str>,
) -> Result<&'static dyn finstack_core::dates::HolidayCalendar> {
    if let Some(id) = calendar_id {
        return CalendarRegistry::global().resolve_str(id).ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::InputError::NotFound {
                id: format!("calendar '{}'", id),
            })
        });
    }

    // Explicitly use weekends-only calendar when no calendar ID is provided.
    Ok(&WEEKENDS_ONLY)
}

/// Adjust a single date using the canonical strict calendar policy.
pub fn adjust_date(
    date: Date,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
) -> Result<Date> {
    let cal = resolve_calendar_strict(calendar_id)?;
    adjust(date, bdc, cal)
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
    /// Optional holiday calendar identifier.
    pub calendar_id: Option<&'a str>,
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
///
/// # Calendar Policy (Strict)
/// - If `calendar_id` is provided but cannot be resolved, returns an error.
/// - If `calendar_id` is `None`, a weekends-only calendar is used so BDC is still applied.
pub fn build_periods(params: BuildPeriodsParams<'_>) -> Result<Vec<SchedulePeriod>> {
    let cal = resolve_calendar_strict(params.calendar_id)?;

    let schedule = ScheduleBuilder::new(params.start, params.end)?
        .frequency(params.frequency)
        .stub_rule(params.stub)
        .end_of_month(params.end_of_month)
        .adjust_with(params.bdc, cal)
        .build()?;

    let mut dates = schedule.dates;

    // Ensure maturity date is included when the schedule stops at the penultimate boundary.
    if matches!(dates.last(), Some(last) if *last < params.end) {
        dates.push(params.end);
    }

    if dates.len() < 2 {
        return Ok(Vec::new());
    }

    let dc_ctx = DayCountCtx::default();
    let mut periods = Vec::with_capacity(dates.len().saturating_sub(1));

    for window in dates.windows(2) {
        let accrual_start = window[0];
        let accrual_end = window[1];

        let payment_date = if params.payment_lag_days == 0 {
            accrual_end
        } else {
            accrual_end.add_business_days(params.payment_lag_days, cal)?
        };

        let reset_date = params
            .reset_lag_days
            .map(|lag| {
                if lag == 0 {
                    Ok(accrual_start)
                } else {
                    accrual_start.add_business_days(-lag, cal)
                }
            })
            .transpose()?;

        let accrual_year_fraction =
            params
                .day_count
                .year_fraction(accrual_start, accrual_end, dc_ctx)?;

        periods.push(SchedulePeriod {
            accrual_start,
            accrual_end,
            payment_date,
            reset_date,
            accrual_year_fraction,
        });
    }

    Ok(periods)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("valid month"), day)
            .expect("valid date")
    }

    #[test]
    fn adjust_date_weekend_following_uses_weekends_only() {
        let sat = d(2025, 1, 4);
        let adj = adjust_date(sat, BusinessDayConvention::Following, None)
            .expect("adjust should succeed");
        assert_eq!(adj, d(2025, 1, 6));
    }

    #[test]
    fn build_periods_end_of_month_rolls() {
        let start = d(2025, 1, 31);
        let end = d(2025, 4, 30);
        let periods = build_periods(BuildPeriodsParams {
            start,
            end,
            frequency: Tenor::monthly(),
            stub: StubKind::None,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            end_of_month: true,
            day_count: DayCount::Act360,
            payment_lag_days: 0,
            reset_lag_days: None,
        })
        .expect("schedule");

        assert_eq!(periods.len(), 3);
        assert_eq!(periods[0].accrual_end, d(2025, 2, 28));
        assert_eq!(periods[2].accrual_end, end);
    }

    #[test]
    fn build_periods_payment_lag_is_forward() {
        let periods = build_periods(BuildPeriodsParams {
            start: d(2025, 1, 15),
            end: d(2025, 7, 15),
            frequency: Tenor::quarterly(),
            stub: StubKind::None,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            end_of_month: false,
            day_count: DayCount::Act360,
            payment_lag_days: 2,
            reset_lag_days: Some(2),
        })
        .expect("schedule");

        assert_eq!(periods.len(), 2);
        assert!(periods[0].payment_date >= periods[0].accrual_end);
    }

    #[test]
    fn build_periods_errors_on_unknown_calendar() {
        let res = build_periods(BuildPeriodsParams {
            start: d(2025, 1, 15),
            end: d(2025, 7, 15),
            frequency: Tenor::quarterly(),
            stub: StubKind::None,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("NOT_A_CAL"),
            end_of_month: false,
            day_count: DayCount::Act360,
            payment_lag_days: 0,
            reset_lag_days: None,
        });
        assert!(res.is_err());
    }
}
