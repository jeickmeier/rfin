//! Helpers for building common date schedules.

use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{BusinessDayConvention, Date, Frequency, ScheduleBuilder, StubKind};

/// Builds a schedule matching the bond's actual payment frequency.
///
/// This is the market standard method for computing par swap rates and ASW.
pub fn build_bond_schedule(
    as_of: Date,
    maturity: Date,
    freq: Frequency,
    stub: StubKind,
    bdc: BusinessDayConvention,
    calendar_id: Option<&str>,
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
