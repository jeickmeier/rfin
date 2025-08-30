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
/// See unit tests and `examples/` for usage.
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


