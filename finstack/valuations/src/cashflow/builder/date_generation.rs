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

use finstack_core::dates::{BusinessDayConvention, Date, ScheduleBuilder, StubKind, Tenor};

/// Period schedule with helper maps/flags for coupon generation.
/// Period schedule output with business day adjustment tracking.
#[derive(Clone, Debug)]
pub struct PeriodSchedule {
    /// Generated sequence of payment dates
    pub dates: Vec<Date>,
    /// Mapping from adjusted dates to unadjusted dates
    pub prev: finstack_core::HashMap<Date, Date>,
    /// Set of payment dates that correspond to first or last periods.
    pub first_or_last: finstack_core::HashSet<Date>,
}

impl PeriodSchedule {
    /// Build prev map and first_or_last set from dates.
    fn from_dates(dates: Vec<Date>) -> Self {
        let mut prev = finstack_core::HashMap::default();
        prev.reserve(dates.len());
        if let Some(&first) = dates.first() {
            let mut p = first;
            for &d in dates.iter().skip(1) {
                prev.insert(d, p);
                p = d;
            }
        }

        let mut first_or_last = finstack_core::HashSet::default();
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

/// Internal implementation for schedule building with strict error handling.
fn build_dates_impl(
    start: Date,
    end: Date,
    freq: Tenor,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
) -> finstack_core::Result<PeriodSchedule> {
    let mut builder = ScheduleBuilder::new(start, end)?
        .frequency(freq)
        .stub_rule(stub);

    // Configure calendar adjustment if provided
    if let Some(id) = calendar_id {
        builder = builder.adjust_with_id(bdc, id);
    }

    let schedule = builder.build()?;
    Ok(PeriodSchedule::from_dates(schedule.dates))
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
///     None,
/// )?;
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
) -> finstack_core::Result<PeriodSchedule> {
    build_dates_impl(start, end, freq, stub, bdc, calendar_id)
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
            Some("NOT_A_CAL"),
        );
        assert!(res.is_err());
    }
}
