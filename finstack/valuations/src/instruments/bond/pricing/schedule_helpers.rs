//! Helpers for building common date schedules.

use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{BusinessDayConvention, Date, Frequency, ScheduleBuilder, StubKind};

/// Builds a simple annual schedule from a start date to an end date.
///
/// This is a common requirement for approximating par swap rates in metrics
/// like I-Spread and ASW Spread when a full swap curve is not available.
pub fn build_annual_schedule(as_of: Date, maturity: Date) -> Vec<Date> {
    let mut dates: Vec<Date> = vec![as_of];
    let mut y = as_of.year();
    while y < maturity.year() {
        // increment by 1Y on the same day/month if possible
        let next = Date::from_calendar_date(y + 1, as_of.month(), as_of.day()).unwrap_or(maturity);
        dates.push(next);
        y += 1;
        if next >= maturity {
            break;
        }
    }
    if !dates.is_empty() && *dates.last().unwrap() < maturity {
        dates.push(maturity);
    }
    dates
}

/// Builds a schedule matching the bond's actual payment frequency.
///
/// This is the market standard method for computing par swap rates and ASW.
pub fn build_bond_schedule(
    as_of: Date,
    maturity: Date,
    freq: Frequency,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&'static str>,
) -> Vec<Date> {
    let builder = ScheduleBuilder::new(as_of, maturity)
        .frequency(freq)
        .stub_rule(stub);

    let schedule = if let Some(id) = calendar_id {
        if let Some(cal) = calendar_by_id(id) {
            builder.adjust_with(bdc, cal).build()
        } else {
            builder.build()
        }
    } else {
        builder.build()
    };

    match schedule {
        Ok(sched) => sched.into_iter().collect(),
        Err(_) => vec![as_of, maturity],
    }
}
