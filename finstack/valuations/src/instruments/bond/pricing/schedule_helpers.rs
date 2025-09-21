//! Helpers for building common date schedules.

use finstack_core::dates::Date;

/// Builds a simple annual schedule from a start date to an end date.
///
/// This is a common requirement for approximating par swap rates in metrics
/// like I-Spread and ASW Spread when a full swap curve is not available.
pub fn build_annual_schedule(as_of: Date, maturity: Date) -> Vec<Date> {
    let mut dates: Vec<Date> = vec![as_of];
    let mut y = as_of.year();
    while y < maturity.year() {
        // increment by 1Y on the same day/month if possible
        let next = Date::from_calendar_date(y + 1, as_of.month(), as_of.day())
            .unwrap_or(maturity);
        dates.push(next);
        y += 1;
        if next >= maturity { break; }
    }
    if !dates.is_empty() && *dates.last().unwrap() < maturity {
        dates.push(maturity);
    }
    dates
}
