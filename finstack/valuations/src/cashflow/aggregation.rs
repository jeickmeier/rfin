#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use hashbrown::HashMap;

/// A single dated cashflow (date, money). Generic across instruments.
pub type DatedFlow = (Date, Money);

/// Currency-preserving aggregation of cashflows into `Period`s.
/// Returns map: PeriodId -> (Currency -> amount).
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


