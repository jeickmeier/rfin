//! Currency-preserving aggregation of cashflows into `Period`s.

use finstack_core::prelude::*;
use finstack_core::F;
use indexmap::IndexMap;

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
) -> IndexMap<PeriodId, IndexMap<Currency, F>> {
    use core::cmp::Ordering;
    let mut out: IndexMap<PeriodId, IndexMap<Currency, F>> = IndexMap::new();

    if flows.is_empty() || periods.is_empty() {
        return out;
    }

    // Sort flows by date once. Do not mutate caller slice.
    let mut sorted: Vec<DatedFlow> = flows.to_vec();
    sorted.sort_by(|(d1, _), (d2, _)| d1.cmp(d2));

    // For each period (preserve input order), locate the first flow >= start via binary search,
    // then accumulate until flow.date < end. This is O(m log n + total_matched_flows).
    for p in periods {
        // Find lower bound index for p.start
        let mut lo = 0usize;
        let mut hi = sorted.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            match sorted[mid].0.cmp(&p.start) {
                Ordering::Less => lo = mid + 1,
                _ => hi = mid,
            }
        }

        let mut per_ccy: IndexMap<Currency, F> = IndexMap::new();
        let mut i = lo;
        while i < sorted.len() {
            let (d, m) = sorted[i];
            if d >= p.end { break; }
            // We know d >= p.start by construction of lower bound
            let e = per_ccy.entry(m.currency()).or_insert(0.0);
            *e += m.amount();
            i += 1;
        }
        if !per_ccy.is_empty() {
            out.insert(p.id, per_ccy);
        }
    }
    out
}
