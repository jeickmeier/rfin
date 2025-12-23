//! Date and schedule generation utilities.
//!
//! This module provides functions for generating payment date schedules based on
//! frequency, stub rules, and business day conventions. It supports both strict
//! and graceful error handling modes.
//!
//! ## Responsibilities
//!
//! - Generate period schedules with `build_dates` (graceful fallback)
//! - Generate period schedules with `build_dates_checked` (strict validation)
//! - Create `PeriodSchedule` with helper maps for previous date lookups
//! - Apply business day adjustments using calendars

use finstack_core::dates::{BusinessDayConvention, Date, ScheduleBuilder, StubKind, Tenor};

/// Period schedule with helper maps/flags for coupon generation.
/// Period schedule output with business day adjustment tracking.
#[derive(Clone, Debug)]
pub struct PeriodSchedule {
    /// Generated sequence of payment dates
    pub dates: Vec<Date>,
    /// Mapping from adjusted dates to unadjusted dates
    pub prev: finstack_core::collections::HashMap<Date, Date>,
    /// Set of payment dates that correspond to first or last periods.
    pub first_or_last: finstack_core::collections::HashSet<Date>,
}

impl PeriodSchedule {
    /// Build prev map and first_or_last set from dates.
    fn from_dates(dates: Vec<Date>) -> Self {
        let mut prev = finstack_core::collections::HashMap::default();
        prev.reserve(dates.len());
        if let Some(&first) = dates.first() {
            let mut p = first;
            for &d in dates.iter().skip(1) {
                prev.insert(d, p);
                p = d;
            }
        }

        let mut first_or_last = finstack_core::collections::HashSet::default();
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
/// When `strict` is true, errors are propagated; when false, graceful fallback to empty schedule.
fn build_dates_impl(
    start: Date,
    end: Date,
    freq: Tenor,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
    strict: bool,
) -> Result<PeriodSchedule, ScheduleError> {
    let mut builder = match ScheduleBuilder::try_new(start, end) {
        Ok(b) => b.frequency(freq).stub_rule(stub),
        Err(e) => {
            // In non-strict mode, return an empty schedule rather than bubbling up.
            // This matches the "graceful fallback" contract and avoids panicking on
            // invalid ranges (e.g., start > end).
            if strict {
                return Err(ScheduleError::Core(e));
            }
            return Ok(PeriodSchedule::from_dates(vec![]));
        }
    };

    // Configure calendar adjustment if provided
    if let Some(id) = calendar_id {
        // In non-strict mode, fall back to an unadjusted schedule if the calendar is missing.
        // This preserves historical behavior and matches the CashFlowBuilder contract.
        if !strict {
            builder = builder.allow_missing_calendar(true);
        }
        builder = builder.adjust_with_id(bdc, id);
    }

    // Enable graceful mode for non-strict builds
    if !strict {
        builder = builder.graceful_fallback(true);
    }

    let schedule = builder.build()?;
    Ok(PeriodSchedule::from_dates(schedule.dates))
}

/// Build a schedule between start/end with graceful error handling.
///
/// This is the **unchecked variant** that provides graceful fallback behavior:
/// - If schedule building fails, returns empty schedule `[]`
/// - If calendar is not found, generates schedule without business day adjustment
/// - Never panics, always returns a valid `PeriodSchedule`
///
/// For strict validation that returns errors, use [`build_dates_checked`].
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
/// let sched = build_dates(start, end, Tenor::quarterly(), StubKind::None, BusinessDayConvention::Following, None);
/// assert!(sched.dates.len() >= 2);
/// # Ok::<(), finstack_core::Error>(())
/// ```
pub fn build_dates(
    start: Date,
    end: Date,
    freq: Tenor,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
) -> PeriodSchedule {
    // Never panics - uses graceful fallback on errors
    match build_dates_impl(start, end, freq, stub, bdc, calendar_id, false) {
        Ok(s) => s,
        Err(_) => PeriodSchedule::from_dates(vec![]),
    }
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
/// use finstack_core::dates::{Date, Tenor, BusinessDayConvention, StubKind, create_date};
/// use finstack_valuations::cashflow::builder::date_generation::build_dates_checked;
/// use time::Month;
///
/// let start = create_date(2025, Month::January, 15)?;
/// let end = create_date(2025, Month::July, 15)?;
/// let sched = build_dates_checked(
///     start, end,
///     Tenor::quarterly(),
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
    freq: Tenor,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
) -> Result<PeriodSchedule, ScheduleError> {
    build_dates_impl(start, end, freq, stub, bdc, calendar_id, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).expect("Valid month (1-12)"), day)
            .expect("Valid test date")
    }

    #[test]
    fn build_dates_checked_errors_on_unknown_calendar() {
        let res = build_dates_checked(
            d(2025, 1, 1),
            d(2025, 4, 1),
            Tenor::quarterly(),
            StubKind::None,
            BusinessDayConvention::Following,
            Some("NOT_A_CAL"),
        );
        assert!(res.is_err());
    }
}
