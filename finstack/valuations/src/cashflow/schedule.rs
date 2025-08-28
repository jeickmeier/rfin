#![allow(missing_docs)]

use finstack_core::dates::{Date, Frequency, BusinessDayConvention, StubKind, ScheduleBuilder};
use finstack_core::dates::holiday::calendars::calendar_by_id;

/// Period schedule with helper maps/flags for coupon generation.
#[derive(Clone, Debug)]
pub struct PeriodSchedule {
    pub dates: Vec<Date>,
    pub prev: hashbrown::HashMap<Date, Date>,
    /// Set of payment dates that correspond to first or last periods.
    pub first_or_last: hashbrown::HashSet<Date>,
}

/// Build a schedule between start/end with standard adjustments and stub rule.
pub fn build_dates(
    start: Date,
    end: Date,
    freq: Frequency,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&'static str>,
) -> PeriodSchedule {
    let builder = ScheduleBuilder::new(start, end)
        .frequency(freq)
        .stub_rule(stub);

    let dates: Vec<Date> = if let Some(id) = calendar_id {
        if let Some(cal) = calendar_by_id(id) {
            builder.adjust_with(bdc, cal).build().collect()
        } else {
            builder.build_raw().collect()
        }
    } else {
        builder.build_raw().collect()
    };

    let mut prev = hashbrown::HashMap::with_capacity(dates.len());
    let mut p = dates[0];
    for &d in dates.iter().skip(1) { prev.insert(d, p); p = d; }

    let mut first_or_last: hashbrown::HashSet<Date> = hashbrown::HashSet::new();
    if dates.len() >= 2 {
        first_or_last.insert(dates[1]);
        if let Some(&last) = dates.last() { first_or_last.insert(last); }
    }

    PeriodSchedule { dates, prev, first_or_last }
}


