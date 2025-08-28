//! Currency-preserving aggregation of cashflows into `Period`s.

use finstack_core::prelude::*;
use finstack_core::F;
use hashbrown::HashMap;

/// A single dated cashflow (date, money). Generic across instruments.
/// 
/// Used for aggregation and NPV calculations where only date and amount matter.
pub type DatedFlow = (Date, Money);

/// Currency-preserving aggregation of cashflows into `Period`s.
/// 
/// Groups cashflows by time period while preserving currency separation.
/// Returns a map: `PeriodId -> (Currency -> amount)`.
/// 
/// # Example
/// ```rust
/// use finstack_core::dates::{Date, Period, PeriodId};
/// use finstack_core::currency::Currency;
/// use finstack_core::money::Money;
/// use finstack_valuations::cashflow::aggregation::aggregate_by_period;
/// use time::Month;
/// 
/// let flows = vec![
///     (Date::from_calendar_date(2025, Month::June, 15).unwrap(), Money::new(25_000.0, Currency::USD)),
///     (Date::from_calendar_date(2025, Month::December, 15).unwrap(), Money::new(25_000.0, Currency::USD)),
/// ];
/// let periods = vec![
///     Period { id: PeriodId::half(2025, 1), start: Date::from_calendar_date(2025, Month::January, 1).unwrap(), end: Date::from_calendar_date(2025, Month::July, 1).unwrap(), is_actual: false },
///     Period { id: PeriodId::half(2025, 2), start: Date::from_calendar_date(2025, Month::July, 1).unwrap(), end: Date::from_calendar_date(2026, Month::January, 1).unwrap(), is_actual: false },
/// ];
/// let aggregated = aggregate_by_period(&flows, &periods);
/// ```
pub fn aggregate_by_period(
    flows: &[DatedFlow],
    periods: &[Period],
) -> HashMap<PeriodId, HashMap<Currency, F>> {
    let mut out: HashMap<PeriodId, HashMap<Currency, F>> = HashMap::new();
    for p in periods {
        let mut per_ccy: HashMap<Currency, F> = HashMap::new();
        for (d, m) in flows {
            if *d >= p.start && *d < p.end {
                let e = per_ccy.entry(m.currency()).or_insert(0.0);
                *e += m.amount();
            }
        }
        if !per_ccy.is_empty() {
            out.insert(p.id, per_ccy);
        }
    }
    out
}


