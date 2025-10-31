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

/// Build a schedule between start/end with standard adjustments and stub rule.
///
/// Example
/// -------
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
    let builder = ScheduleBuilder::new(start, end)
        .frequency(freq)
        .stub_rule(stub);

    let dates: Vec<Date> = if let Some(id) = calendar_id {
        if let Some(cal) = calendar_by_id(id) {
            builder
                .adjust_with(bdc, cal)
                .build()
                .expect("Failed to build schedule with calendar adjustment")
                .into_iter()
                .collect()
        } else {
            builder
                .build()
                .expect("Failed to build schedule")
                .into_iter()
                .collect()
        }
    } else {
        builder
            .build()
            .expect("Failed to build schedule")
            .into_iter()
            .collect()
    };

    let mut prev = hashbrown::HashMap::with_capacity(dates.len());
    if let Some(&first) = dates.first() {
        let mut p = first;
        for &d in dates.iter().skip(1) {
            prev.insert(d, p);
            p = d;
        }
    }

    // Mark first and last dates
    let mut first_or_last = hashbrown::HashSet::new();
    if let Some(&first) = dates.first() {
        first_or_last.insert(first);
    }
    if let Some(&last) = dates.last() {
        first_or_last.insert(last);
    }

    PeriodSchedule {
        dates,
        prev,
        first_or_last,
    }
}
