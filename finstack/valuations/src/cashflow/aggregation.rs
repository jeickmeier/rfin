//! Currency-preserving aggregation of cashflows into `Period`s.

use finstack_core::prelude::*;

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
#[inline(never)]
pub fn aggregate_by_period(
    flows: &[DatedFlow],
    periods: &[Period],
) -> IndexMap<PeriodId, IndexMap<Currency, f64>> {
    use core::cmp::Ordering;
    let mut out: IndexMap<PeriodId, IndexMap<Currency, f64>> = IndexMap::new();

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

        let mut per_ccy: IndexMap<Currency, f64> = IndexMap::new();
        let mut i = lo;
        while i < sorted.len() {
            let (d, m) = sorted[i];
            if d >= p.end {
                break;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, Period, PeriodId};
    use time::Month;

    fn d(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    }

    fn quarters_2025() -> Vec<Period> {
        vec![
            Period {
                id: PeriodId::quarter(2025, 1),
                start: d(2025, 1, 1),
                end: d(2025, 4, 1),
                is_actual: true,
            },
            Period {
                id: PeriodId::quarter(2025, 2),
                start: d(2025, 4, 1),
                end: d(2025, 7, 1),
                is_actual: false,
            },
            Period {
                id: PeriodId::quarter(2025, 3),
                start: d(2025, 7, 1),
                end: d(2025, 10, 1),
                is_actual: false,
            },
        ]
    }

    #[test]
    fn empty_inputs_yield_empty_aggregation() {
        let periods = quarters_2025();
        assert!(aggregate_by_period(&[], &periods).is_empty());
        let flows = vec![(d(2025, 1, 15), Money::new(1.0, Currency::USD))];
        assert!(aggregate_by_period(&flows, &[]).is_empty());
    }

    #[test]
    fn cashflows_are_grouped_by_period_and_currency() {
        let periods = quarters_2025();
        let flows = vec![
            // Unsorted on purpose (algorithm should sort internally)
            (d(2025, 4, 15), Money::new(50.0, Currency::USD)),
            (d(2025, 1, 10), Money::new(100.0, Currency::USD)),
            (d(2025, 2, 20), Money::new(200.0, Currency::EUR)),
            // Boundary case: falls exactly on period end, should roll into next quarter
            (d(2025, 4, 1), Money::new(10.0, Currency::USD)),
        ];

        let aggregated = aggregate_by_period(&flows, &periods);
        let expected_keys = vec![PeriodId::quarter(2025, 1), PeriodId::quarter(2025, 2)];
        let keys: Vec<_> = aggregated.keys().cloned().collect();
        assert_eq!(keys, expected_keys);

        let q1 = aggregated.get(&PeriodId::quarter(2025, 1)).unwrap();
        assert_eq!(q1.len(), 2);
        assert!((q1[&Currency::USD] - 100.0).abs() < 1e-12);
        assert!((q1[&Currency::EUR] - 200.0).abs() < 1e-12);

        let q2 = aggregated.get(&PeriodId::quarter(2025, 2)).unwrap();
        assert_eq!(q2.len(), 1);
        assert!((q2[&Currency::USD] - 60.0).abs() < 1e-12);

        // Third quarter has no flows -> should not be present
        assert!(aggregated.get(&PeriodId::quarter(2025, 3)).is_none());
    }
}
