//! Utilities for building schedules.
//!
//! `build_dates` creates a period schedule between start/end using a frequency
//! and stub rule, with optional business-day adjustment by calendar.
//! It returns `PeriodSchedule` with helper maps for previous date lookups and
//! flags for first/last periods to aid stub classification.

use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{BusinessDayConvention, Date, Frequency, ScheduleBuilder, StubKind};

/// Period schedule with helper maps/flags for coupon generation.
#[derive(Clone, Debug)]
pub struct PeriodSchedule {
    pub dates: Vec<Date>,
    pub prev: hashbrown::HashMap<Date, Date>,
    /// Set of payment dates that correspond to first or last periods.
    pub first_or_last: hashbrown::HashSet<Date>,
}

impl PeriodSchedule {
    /// Build prev map and first_or_last set from dates.
    fn from_dates(dates: Vec<Date>) -> Self {
        let mut prev = hashbrown::HashMap::with_capacity(dates.len());
        if let Some(&first) = dates.first() {
            let mut p = first;
            for &d in dates.iter().skip(1) {
                prev.insert(d, p);
                p = d;
            }
        }

        let mut first_or_last = hashbrown::HashSet::new();
        if let Some(&first) = dates.first() {
            first_or_last.insert(first);
        }
        if let Some(&last) = dates.last() {
            first_or_last.insert(last);
        }

        Self {
            dates,
            prev,
            first_or_last,
        }
    }
}

/// Error type for schedule building operations.
#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    /// Core date/time error
    #[error("Schedule building error: {0}")]
    Core(#[from] finstack_core::Error),
}

impl From<ScheduleError> for finstack_core::Error {
    fn from(err: ScheduleError) -> Self {
        match err {
            ScheduleError::Core(core_err) => core_err,
        }
    }
}

/// Internal implementation for schedule building with configurable error handling.
///
/// When `strict` is true, errors are propagated; when false, graceful fallback to [start, end].
fn build_dates_impl(
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
    strict: bool,
) -> Result<PeriodSchedule, ScheduleError> {
    let builder = ScheduleBuilder::new(start, end)
        .frequency(freq)
        .stub_rule(stub);

    let dates: Vec<Date> = if let Some(id) = calendar_id {
        // Calendar specified - try to look it up
        match calendar_by_id(id) {
            Some(cal) => builder.adjust_with(bdc, cal).build()?.into_iter().collect(),
            None => {
                if strict {
                    return Err(ScheduleError::Core(
                        finstack_core::error::InputError::NotFound { id: id.to_string() }.into(),
                    ));
                }
                // Non-strict: fallback to unadjusted schedule
                builder
                    .build()
                    .map(|s| s.into_iter().collect())
                    .unwrap_or_else(|_| vec![start, end])
            }
        }
    } else {
        // No calendar - build unadjusted schedule
        if strict {
            builder.build()?.into_iter().collect()
        } else {
            builder
                .build()
                .map(|s| s.into_iter().collect())
                .unwrap_or_else(|_| vec![start, end])
        }
    };

    Ok(PeriodSchedule::from_dates(dates))
}

/// Build a schedule between start/end with graceful error handling.
///
/// This is the **unchecked variant** that provides graceful fallback behavior:
/// - If schedule building fails, returns minimal schedule `[start, end]`
/// - If calendar is not found, attempts unadjusted schedule
/// - Never panics, always returns a valid `PeriodSchedule`
///
/// For strict validation that returns errors, use [`build_dates_checked`].
///
/// # Example
///
/// ```rust
/// use finstack_core::dates::{Date, Frequency, BusinessDayConvention, StubKind, create_date};
/// use finstack_valuations::cashflow::builder::schedule_utils::build_dates;
/// use time::Month;
///
/// let start = create_date(2025, Month::January, 15)?;
/// let end = create_date(2025, Month::July, 15)?;
/// let sched = build_dates(start, end, Frequency::quarterly(), StubKind::None, BusinessDayConvention::Following, None);
/// assert!(sched.dates.len() >= 2);
/// # Ok::<(), finstack_core::Error>(())
/// ```
pub fn build_dates(
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
) -> PeriodSchedule {
    // Never panics - uses graceful fallback on errors
    build_dates_impl(start, end, freq, stub, bdc, calendar_id, false)
        .expect("build_dates_impl with strict=false should never fail")
}

/// Build a schedule between start/end with strict error handling.
///
/// This is the **checked variant** that propagates all errors:
/// - Returns error if schedule building fails
/// - Returns error if calendar is specified but not found
/// - Returns error if ScheduleBuilder fails for any reason
///
/// For graceful fallback behavior, use [`build_dates`].
///
/// # Errors
///
/// Returns `ScheduleError` when:
/// - `calendar_id` is provided but calendar is not found
/// - Schedule generation fails due to invalid date ranges
/// - Business day adjustment fails
///
/// # Example
///
/// ```rust
/// use finstack_core::dates::{Date, Frequency, BusinessDayConvention, StubKind, create_date};
/// use finstack_valuations::cashflow::builder::schedule_utils::build_dates_checked;
/// use time::Month;
///
/// let start = create_date(2025, Month::January, 15)?;
/// let end = create_date(2025, Month::July, 15)?;
/// let sched = build_dates_checked(
///     start, end,
///     Frequency::quarterly(),
///     StubKind::None,
///     BusinessDayConvention::Following,
///     None
/// )?;
/// assert!(sched.dates.len() >= 2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn build_dates_checked(
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
) -> Result<PeriodSchedule, ScheduleError> {
    build_dates_impl(start, end, freq, stub, bdc, calendar_id, true)
}

/// Build periods from payment schedule dates for PV aggregation.
///
/// Creates Period objects with synthetic IDs based on payment frequency.
/// Each period spans from one payment date (exclusive start) to the next (inclusive end).
///
/// # Arguments
/// * `payment_dates` - Ordered vector of payment dates
/// * `frequency` - Payment frequency to derive period kind
///
/// # Returns
/// Vector of Period objects suitable for cashflow aggregation
pub fn build_periods_from_payment_dates(
    payment_dates: &[Date],
    frequency: Frequency,
) -> Vec<finstack_core::dates::Period> {
    use finstack_core::dates::{Period, PeriodId, PeriodKind};

    if payment_dates.len() < 2 {
        return Vec::new();
    }

    let mut periods = Vec::with_capacity(payment_dates.len() - 1);

    // Determine period kind from frequency
    let period_kind = match frequency {
        Frequency::Months(12) => PeriodKind::Annual,
        Frequency::Months(6) => PeriodKind::SemiAnnual,
        Frequency::Months(3) => PeriodKind::Quarterly,
        Frequency::Months(1) => PeriodKind::Monthly,
        Frequency::Days(7) => PeriodKind::Weekly,
        _ => PeriodKind::Quarterly, // Default fallback
    };

    for i in 0..(payment_dates.len() - 1) {
        let start = payment_dates[i];
        let end = payment_dates[i + 1];

        // Create a synthetic PeriodId based on the start date and frequency
        let period_id = match period_kind {
            PeriodKind::Quarterly => {
                let year = start.year();
                let month = start.month() as u8;
                let quarter = match month {
                    1..=3 => 1,
                    4..=6 => 2,
                    7..=9 => 3,
                    _ => 4,
                };
                PeriodId::quarter(year, quarter)
            }
            PeriodKind::Monthly => {
                let year = start.year();
                let month = start.month() as u8;
                PeriodId::month(year, month)
            }
            PeriodKind::SemiAnnual => {
                let year = start.year();
                let month = start.month() as u8;
                let half = if month <= 6 { 1 } else { 2 };
                PeriodId::half(year, half)
            }
            PeriodKind::Annual => {
                let year = start.year();
                PeriodId::annual(year)
            }
            PeriodKind::Weekly => {
                // For weekly, use a simple week number based on days since start of year
                let year = start.year();
                let year_start = Date::from_calendar_date(year, time::Month::January, 1).unwrap();
                let days = (start - year_start).whole_days();
                let week = ((days / 7) + 1).min(52) as u8;
                PeriodId::week(year, week)
            }
        };

        periods.push(Period {
            id: period_id,
            start,
            end,
            is_actual: false, // All periods are forecast for pricing
        });
    }

    periods
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
    }

    #[test]
    fn build_dates_checked_errors_on_unknown_calendar() {
        let res = build_dates_checked(
            d(2025, 1, 1),
            d(2025, 4, 1),
            Frequency::quarterly(),
            StubKind::None,
            BusinessDayConvention::Following,
            Some("NOT_A_CAL"),
        );
        assert!(res.is_err());
    }
}
