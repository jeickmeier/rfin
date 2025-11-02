//! Currency-preserving aggregation of cashflows into `Period`s.

use finstack_core::prelude::*;
// use crate::cashflow::DatedFlow; // brought into scope by re-export below

use indexmap::IndexMap;

// Re-export to preserve existing import paths in benches and callers
pub use crate::cashflow::DatedFlow;

/// Currency-preserving aggregation of cashflows into `Period`s.
///
/// Groups cashflows by time period while preserving currency separation.
/// Returns a map: `PeriodId -> (Currency -> amount)`.
///
/// See unit tests and `examples/` for usage.
fn aggregate_by_period_sorted(
    sorted: &[DatedFlow],
    periods: &[Period],
) -> IndexMap<PeriodId, IndexMap<Currency, f64>> {
    use core::cmp::Ordering;
    let mut out: IndexMap<PeriodId, IndexMap<Currency, f64>> = IndexMap::new();

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

#[inline(never)]
pub fn aggregate_by_period(
    flows: &[DatedFlow],
    periods: &[Period],
) -> IndexMap<PeriodId, IndexMap<Currency, f64>> {
    let mut sorted: Vec<DatedFlow> = flows.to_vec();
    if sorted.is_empty() || periods.is_empty() {
        return IndexMap::new();
    }
    sorted.sort_by(|(d1, _), (d2, _)| d1.cmp(d2));
    aggregate_by_period_sorted(&sorted, periods)
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

// =============================================================================
// Precision-Preserving Aggregation (Market Standards Review - Priority 3)
// =============================================================================

use finstack_core::math::summation::kahan_sum;

/// Threshold for switching to Kahan summation (number of cashflows).
///
/// For legs with ≤ 20 cashflows, naive summation is used (fast path).
/// For legs with > 20 cashflows, Kahan summation is used (precision-preserving).
pub const KAHAN_THRESHOLD: usize = 20;

/// Aggregate simple cashflow amounts using precision-preserving summation.
///
/// For cashflow legs with more than 20 flows, this function uses Kahan
/// summation to prevent floating-point rounding errors that accumulate
/// in naive summation. This is especially important for:
/// - Long-maturity bonds (30Y+)
/// - CLO/ABS waterfalls with monthly payments
/// - Swap legs with high frequency (monthly, weekly)
///
/// # Examples
///
/// ```
/// use finstack_core::dates::Date;
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use finstack_valuations::cashflow::aggregation::aggregate_cashflows_precise;
/// use time::Month;
///
/// let flows = vec![
///     (Date::from_calendar_date(2025, Month::January, 1).unwrap(), Money::new(1000.0, Currency::USD)),
///     (Date::from_calendar_date(2025, Month::February, 1).unwrap(), Money::new(1000.0, Currency::USD)),
///     (Date::from_calendar_date(2025, Month::March, 1).unwrap(), Money::new(1000.0, Currency::USD)),
/// ];
///
/// let total = aggregate_cashflows_precise(&flows);
/// assert_eq!(total.amount(), 3000.0);
/// ```
/// Note: For empty input, returns 0.0 in USD to preserve `Money` typing
/// without inferring currency. Callers needing explicit currency should
/// wrap or provide one.
pub fn aggregate_cashflows_precise(flows: &[DatedFlow]) -> Money {
    if flows.is_empty() {
        return Money::new(0.0, Currency::USD); // Default currency
    }

    let currency = flows[0].1.currency();
    let len = flows.len();

    let total = if len > KAHAN_THRESHOLD {
        // Use Kahan summation for long legs (precision-preserving)
        kahan_sum(flows.iter().map(|(_, m)| m.amount()))
    } else {
        // Fast path for short legs
        flows.iter().map(|(_, m)| m.amount()).sum()
    };

    Money::new(total, currency)
}

#[cfg(test)]
mod precision_tests {
    use super::*;
    use time::Month;

    #[test]
    fn aggregation_empty_returns_zero_usd() {
        let total = aggregate_cashflows_precise(&[]);
        assert_eq!(total.amount(), 0.0);
        assert_eq!(total.currency(), Currency::USD);
    }

    #[test]
    fn test_kahan_vs_naive_30y_bond() {
        // Simulate 30-year semi-annual bond (60 cashflows)
        let flows: Vec<DatedFlow> = (0..60)
            .map(|i| {
                // Semi-annual payments
                let months = i * 6;
                let years = months / 12;
                let remaining_months = months % 12;
                (
                    Date::from_calendar_date(
                        2025 + years,
                        Month::try_from((remaining_months + 1) as u8).unwrap(),
                        1,
                    )
                    .unwrap(),
                    Money::new(25_000.0, Currency::USD), // $25k coupon
                )
            })
            .collect();

        let total = aggregate_cashflows_precise(&flows);

        // Should sum to 60 * $25k = $1.5M
        assert!((total.amount() - 1_500_000.0).abs() < 0.01);
    }

    #[test]
    fn test_kahan_threshold_switching() {
        // Test exactly at threshold (20 flows)
        let flows_at_threshold: Vec<DatedFlow> = (0..20)
            .map(|i| {
                let day = (i % 28) + 1;
                (
                    Date::from_calendar_date(2025, Month::January, day as u8).unwrap(),
                    Money::new(50.0, Currency::USD),
                )
            })
            .collect();

        let total_at = aggregate_cashflows_precise(&flows_at_threshold);
        assert_eq!(total_at.amount(), 1000.0);

        // Test just above threshold (21 flows) - should use Kahan
        let flows_above: Vec<DatedFlow> = (0..21)
            .map(|i| {
                let day = (i % 28) + 1;
                (
                    Date::from_calendar_date(2025, Month::January, day as u8).unwrap(),
                    Money::new(50.0, Currency::USD),
                )
            })
            .collect();

        let total_above = aggregate_cashflows_precise(&flows_above);
        assert_eq!(total_above.amount(), 1050.0);
    }
}
