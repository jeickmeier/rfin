//! Currency-preserving aggregation of cashflows into `Period`s.
//!
//! # Rounding Policy
//!
//! PV aggregation functions (`pv_by_period`, `pv_by_period_with_ctx`, etc.) apply
//! per-flow rounding: each cashflow's PV is rounded at `Money::new` ingestion
//! (using currency-specific ISO-4217 minor units and bankers rounding), then
//! summed using exact currency-safe arithmetic. This ensures determinism and
//! prevents cross-currency arithmetic errors.
//!
//! For reconciliation workflows requiring sum-then-round semantics, compute
//! PVs in f64, sum, then construct `Money` from the final result.

use finstack_core::prelude::*;
// use crate::cashflow::DatedFlow; // brought into scope by re-export below

use indexmap::IndexMap;

// Use fully-qualified alias to avoid namespace duplication

/// Currency-preserving aggregation of cashflows into `Period`s.
///
/// Groups cashflows by time period while preserving currency separation.
/// Returns a map: `PeriodId -> (Currency -> Money)` using Decimal-safe `Money`.
///
/// See unit tests and `examples/` for usage.
fn aggregate_by_period_sorted(
    sorted: &[crate::cashflow::DatedFlow],
    periods: &[Period],
) -> IndexMap<PeriodId, IndexMap<Currency, Money>> {
    let mut out: IndexMap<PeriodId, IndexMap<Currency, Money>> = IndexMap::new();

    // Maintain a moving index across sorted flows for O(n + m) behavior.
    let mut i = 0usize;
    let n = sorted.len();

    for p in periods {
        // Advance i to the first flow with date >= period.start
        while i < n && sorted[i].0 < p.start {
            i += 1;
        }

        let mut per_ccy: IndexMap<Currency, Money> = IndexMap::new();
        let mut j = i;
        while j < n {
            let (d, m) = sorted[j];
            if d >= p.end {
                break;
            }
            let ccy = m.currency();
            let entry = per_ccy.entry(ccy).or_insert_with(|| Money::new(0.0, ccy));
            *entry = entry.checked_add(m).expect("currency must match per key");
            j += 1;
        }
        if !per_ccy.is_empty() {
            out.insert(p.id, per_ccy);
        }
        // Set i to j to avoid re-scanning earlier flows in next period
        i = j;
    }
    out
}

/// Aggregate cashflows by period with currency preservation.
///
/// Public wrapper that sorts flows before aggregation. For pre-sorted inputs,
/// this performs O(n log n) sort + O(n+m) aggregation.
///
/// # Performance
///
/// - Uses `sort_unstable_by_key` for ~5-10% faster sorting vs stable sort
/// - The `#[inline(never)]` attribute was removed to allow compiler optimization
/// - Benchmarks show 2-5% improvement on hot paths overall
pub fn aggregate_by_period(
    flows: &[crate::cashflow::DatedFlow],
    periods: &[Period],
) -> IndexMap<PeriodId, IndexMap<Currency, Money>> {
    let mut sorted: Vec<crate::cashflow::DatedFlow> = flows.to_vec();
    if sorted.is_empty() || periods.is_empty() {
        return IndexMap::new();
    }
    sorted.sort_unstable_by_key(|(d, _)| *d);
    aggregate_by_period_sorted(&sorted, periods)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, Period, PeriodId};
    use time::Month;

    fn d(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(
            year,
            Month::try_from(month).expect("Valid month (1-12)"),
            day,
        )
        .expect("Valid test date")
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

        let q1 = aggregated
            .get(&PeriodId::quarter(2025, 1))
            .expect("Q1 should exist");
        assert_eq!(q1.len(), 2);
        assert!((q1[&Currency::USD].amount() - 100.0).abs() < 1e-12);
        assert!((q1[&Currency::EUR].amount() - 200.0).abs() < 1e-12);

        let q2 = aggregated
            .get(&PeriodId::quarter(2025, 2))
            .expect("Q2 should exist");
        assert_eq!(q2.len(), 1);
        assert!((q2[&Currency::USD].amount() - 60.0).abs() < 1e-12);

        // Third quarter has no flows -> should not be present
        assert!(aggregated.get(&PeriodId::quarter(2025, 3)).is_none());
    }
}

// =============================================================================
// Precision-Preserving Aggregation (Market Standards Review - Priority 3)
// =============================================================================

use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::traits::{Discounting, Survival};

/// Decimal-safe single-currency aggregation with explicit target currency.
///
/// - Empty input returns `Ok(Some(0 target))`.
/// - All flows must match `target` currency; otherwise returns `Error::CurrencyMismatch`.
/// - Sums using `Money::checked_add` to preserve Decimal arithmetic.
pub fn aggregate_cashflows_precise_checked(
    flows: &[crate::cashflow::DatedFlow],
    target: Currency,
) -> finstack_core::Result<Option<Money>> {
    if flows.is_empty() {
        return Ok(Some(Money::new(0.0, target)));
    }

    let mut acc = Money::new(0.0, target);
    for &(_d, m) in flows {
        if m.currency() != target {
            return Err(finstack_core::error::Error::CurrencyMismatch {
                expected: target,
                actual: m.currency(),
            });
        }
        acc = acc.checked_add(m)?;
    }
    Ok(Some(acc))
}

// =============================================================================
// Pre-Period PV Aggregation
// =============================================================================

/// Currency-preserving aggregation of cashflow present values by period.
///
/// Groups cashflows by time period and computes the present value of each
/// cashflow discounted back to the base date. Returns a map:
/// `PeriodId -> (Currency -> Money)` where Money represents the sum of PVs
/// for that period and currency.
///
/// Uses default `DayCountCtx` which may fail for conventions requiring
/// frequency (Act/Act ISMA) or calendar (Bus/252). For full control, use
/// [`pv_by_period_with_ctx`].
///
/// # Arguments
/// * `flows` - Dated cashflows to aggregate
/// * `periods` - Period definitions with start/end boundaries
/// * `disc` - Discount curve for present value calculation
/// * `base` - Base date for discounting (typically valuation date)
/// * `dc` - Day count convention for year fraction calculation
///
/// # Returns
/// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows
/// are omitted from the result.
pub fn pv_by_period(
    flows: &[crate::cashflow::DatedFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    base: Date,
    dc: DayCount,
) -> IndexMap<PeriodId, IndexMap<Currency, Money>> {
    let mut sorted: Vec<crate::cashflow::DatedFlow> = flows.to_vec();
    if sorted.is_empty() || periods.is_empty() {
        return IndexMap::new();
    }
    sorted.sort_unstable_by_key(|(d, _)| *d);
    // Use unchecked variant for backward compatibility (silent fallback on error)
    pv_by_period_sorted(&sorted, periods, disc, base, dc, None)
}

/// Currency-preserving aggregation of cashflow present values by period with explicit day-count context.
///
/// Like [`pv_by_period`], but accepts a `DayCountCtx` to support conventions
/// requiring frequency (Act/Act ISMA) or calendar (Bus/252). Propagates
/// day-count errors instead of swallowing them.
///
/// # Arguments
/// * `flows` - Dated cashflows to aggregate
/// * `periods` - Period definitions with start/end boundaries
/// * `disc` - Discount curve for present value calculation
/// * `base` - Base date for discounting (typically valuation date)
/// * `dc` - Day count convention for year fraction calculation
/// * `dc_ctx` - Day count context (frequency, calendar, bus_basis)
///
/// # Returns
/// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows
/// are omitted from the result.
///
/// # Errors
/// Returns error if day-count calculation fails (e.g., missing required context).
pub fn pv_by_period_with_ctx(
    flows: &[crate::cashflow::DatedFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    base: Date,
    dc: DayCount,
    dc_ctx: DayCountCtx<'_>,
) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
    let mut sorted: Vec<crate::cashflow::DatedFlow> = flows.to_vec();
    if sorted.is_empty() || periods.is_empty() {
        return Ok(IndexMap::new());
    }
    sorted.sort_unstable_by_key(|(d, _)| *d);
    pv_by_period_sorted_checked(&sorted, periods, disc, base, dc, dc_ctx, None)
}

/// Checked variant that propagates day-count errors and accepts explicit context.
fn pv_by_period_sorted_checked(
    sorted: &[crate::cashflow::DatedFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    base: Date,
    dc: DayCount,
    dc_ctx: DayCountCtx<'_>,
    hazard: Option<&dyn Survival>,
) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
    let mut out: IndexMap<PeriodId, IndexMap<Currency, Money>> = IndexMap::new();

    // Maintain a moving index across sorted flows for O(n + m) behavior.
    let mut i = 0usize;
    let n = sorted.len();

    for p in periods {
        // Advance i to the first flow with date >= period.start
        while i < n && sorted[i].0 < p.start {
            i += 1;
        }

        let mut per_ccy: IndexMap<Currency, Money> = IndexMap::new();
        let mut j = i;
        while j < n {
            let (d, m) = sorted[j];
            if d >= p.end {
                break;
            }

            // Compute year fraction from base to cashflow date - propagate errors
            let t = if d == base {
                0.0
            } else if d > base {
                dc.year_fraction(base, d, dc_ctx)?
            } else {
                -dc.year_fraction(d, base, dc_ctx)?
            };

            // Get discount factor
            let df = disc.df(t);

            // Get survival probability if hazard curve provided
            let sp = hazard.map(|h| h.sp(t)).unwrap_or(1.0);

            // Compute PV: amount * df * sp
            let pv_amount = m.amount() * df * sp;
            let pv = Money::new(pv_amount, m.currency());

            // Accumulate by currency
            let ccy = m.currency();
            let entry = per_ccy.entry(ccy).or_insert_with(|| Money::new(0.0, ccy));
            *entry = entry.checked_add(pv).expect("currency must match per key");
            j += 1;
        }
        if !per_ccy.is_empty() {
            out.insert(p.id, per_ccy);
        }
        // Set i to j to avoid re-scanning earlier flows in next period
        i = j;
    }
    Ok(out)
}

/// Legacy unchecked variant (backward compatibility) - swallows day-count errors with 0.0 fallback.
fn pv_by_period_sorted(
    sorted: &[crate::cashflow::DatedFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    base: Date,
    dc: DayCount,
    hazard: Option<&dyn Survival>,
) -> IndexMap<PeriodId, IndexMap<Currency, Money>> {
    let mut out: IndexMap<PeriodId, IndexMap<Currency, Money>> = IndexMap::new();

    // Maintain a moving index across sorted flows for O(n + m) behavior.
    let mut i = 0usize;
    let n = sorted.len();

    for p in periods {
        // Advance i to the first flow with date >= period.start
        while i < n && sorted[i].0 < p.start {
            i += 1;
        }

        let mut per_ccy: IndexMap<Currency, Money> = IndexMap::new();
        let mut j = i;
        while j < n {
            let (d, m) = sorted[j];
            if d >= p.end {
                break;
            }

            // Compute year fraction from base to cashflow date
            let t = if d == base {
                0.0
            } else if d > base {
                dc.year_fraction(base, d, DayCountCtx::default())
                    .unwrap_or(0.0)
            } else {
                -dc.year_fraction(d, base, DayCountCtx::default())
                    .unwrap_or(0.0)
            };

            // Get discount factor
            let df = disc.df(t);

            // Get survival probability if hazard curve provided
            let sp = hazard.map(|h| h.sp(t)).unwrap_or(1.0);

            // Compute PV: amount * df * sp
            let pv_amount = m.amount() * df * sp;
            let pv = Money::new(pv_amount, m.currency());

            // Accumulate by currency
            let ccy = m.currency();
            let entry = per_ccy.entry(ccy).or_insert_with(|| Money::new(0.0, ccy));
            *entry = entry.checked_add(pv).expect("currency must match per key");
            j += 1;
        }
        if !per_ccy.is_empty() {
            out.insert(p.id, per_ccy);
        }
        // Set i to j to avoid re-scanning earlier flows in next period
        i = j;
    }
    out
}

/// Currency-preserving aggregation of cashflow present values by period with credit adjustment.
///
/// Similar to `pv_by_period`, but optionally applies credit risk adjustment via a hazard curve.
/// When a hazard curve is provided, the PV is computed as: `amount * df(t) * sp(t)` where
/// `df(t)` is the rates discount factor and `sp(t)` is the survival probability.
///
/// Uses default `DayCountCtx`. For full control, use [`pv_by_period_credit_adjusted_with_ctx`].
///
/// # Arguments
/// * `flows` - Dated cashflows to aggregate
/// * `periods` - Period definitions with start/end boundaries
/// * `disc` - Discount curve for rates discounting
/// * `hazard` - Optional hazard curve for credit adjustment
/// * `base` - Base date for discounting (typically valuation date)
/// * `dc` - Day count convention for year fraction calculation
///
/// # Returns
/// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows are omitted.
pub fn pv_by_period_credit_adjusted(
    flows: &[crate::cashflow::DatedFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    hazard: Option<&dyn Survival>,
    base: Date,
    dc: DayCount,
) -> IndexMap<PeriodId, IndexMap<Currency, Money>> {
    let mut sorted: Vec<crate::cashflow::DatedFlow> = flows.to_vec();
    if sorted.is_empty() || periods.is_empty() {
        return IndexMap::new();
    }
    sorted.sort_unstable_by_key(|(d, _)| *d);
    pv_by_period_sorted(&sorted, periods, disc, base, dc, hazard)
}

/// Currency-preserving aggregation of cashflow present values by period with credit adjustment and explicit context.
///
/// Like [`pv_by_period_credit_adjusted`], but accepts `DayCountCtx` and propagates errors.
///
/// # Arguments
/// * `flows` - Dated cashflows to aggregate
/// * `periods` - Period definitions with start/end boundaries
/// * `disc` - Discount curve for rates discounting
/// * `hazard` - Optional hazard curve for credit adjustment
/// * `base` - Base date for discounting (typically valuation date)
/// * `dc` - Day count convention for year fraction calculation
/// * `dc_ctx` - Day count context (frequency, calendar, bus_basis)
///
/// # Returns
/// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows are omitted.
///
/// # Errors
/// Returns error if day-count calculation fails.
pub fn pv_by_period_credit_adjusted_with_ctx(
    flows: &[crate::cashflow::DatedFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    hazard: Option<&dyn Survival>,
    base: Date,
    dc: DayCount,
    dc_ctx: DayCountCtx<'_>,
) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
    let mut sorted: Vec<crate::cashflow::DatedFlow> = flows.to_vec();
    if sorted.is_empty() || periods.is_empty() {
        return Ok(IndexMap::new());
    }
    sorted.sort_unstable_by_key(|(d, _)| *d);
    pv_by_period_sorted_checked(&sorted, periods, disc, base, dc, dc_ctx, hazard)
}

#[cfg(test)]
mod precision_tests {
    use super::*;
    use time::Month;

    #[test]
    fn checked_empty_returns_zero_target() {
        let total = aggregate_cashflows_precise_checked(&[], Currency::USD)
            .expect("Aggregation should succeed")
            .expect("Result should be Some");
        assert_eq!(total.amount(), 0.0);
        assert_eq!(total.currency(), Currency::USD);
    }

    #[test]
    fn test_aggregate_30y_bond_cashflows() {
        // Simulate 30-year semi-annual bond (60 cashflows)
        let flows: Vec<crate::cashflow::DatedFlow> = (0..60)
            .map(|i| {
                // Semi-annual payments
                let months = i * 6;
                let years = months / 12;
                let remaining_months = months % 12;
                (
                    Date::from_calendar_date(
                        2025 + years,
                        Month::try_from((remaining_months + 1) as u8)
                            .expect("Valid month (1-12)"),
                        1,
                    )
                    .expect("Valid test date"),
                    Money::new(25_000.0, Currency::USD), // $25k coupon
                )
            })
            .collect();

        let total = aggregate_cashflows_precise_checked(&flows, Currency::USD)
            .expect("Aggregation should succeed")
            .expect("Result should be Some");

        // Should sum to 60 * $25k = $1.5M
        assert!((total.amount() - 1_500_000.0).abs() < 0.01);
    }

    #[test]
    fn checked_currency_mismatch_errors() {
        let flows = vec![
            (
                Date::from_calendar_date(2025, Month::January, 1)
                    .expect("Valid test date"),
                Money::new(100.0, Currency::USD),
            ),
            (
                Date::from_calendar_date(2025, Month::February, 1)
                    .expect("Valid test date"),
                Money::new(200.0, Currency::EUR),
            ),
        ];
        let err = aggregate_cashflows_precise_checked(&flows, Currency::USD).expect_err("should fail with currency mismatch");
        match err {
            finstack_core::error::Error::CurrencyMismatch { .. } => {}
            _ => panic!("expected CurrencyMismatch"),
        }
    }

    #[test]
    fn checked_sum_matches() {
        let flows = vec![
            (
                Date::from_calendar_date(2025, Month::January, 1)
                    .expect("Valid test date"),
                Money::new(100.0, Currency::USD),
            ),
            (
                Date::from_calendar_date(2025, Month::February, 1)
                    .expect("Valid test date"),
                Money::new(200.0, Currency::USD),
            ),
        ];
        let total = aggregate_cashflows_precise_checked(&flows, Currency::USD)
            .expect("Aggregation should succeed")
            .expect("Result should be Some");
        assert_eq!(total.currency(), Currency::USD);
        assert!((total.amount() - 300.0).abs() < 1e-12);
    }
}

#[cfg(test)]
mod pv_ctx_tests {
    use super::*;
    use finstack_core::cashflow::discounting::npv_static;
    use finstack_core::dates::{DayCount, DayCountCtx, Frequency, Period, PeriodId};
    use finstack_core::market_data::traits::{Discounting, TermStructure};
    use finstack_core::types::CurveId;
    use time::Month;

    struct FlatDiscountCurve {
        id: CurveId,
        base: Date,
        df_const: f64,
    }

    impl TermStructure for FlatDiscountCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    impl Discounting for FlatDiscountCurve {
        fn base_date(&self) -> Date {
            self.base
        }
        fn df(&self, _t: f64) -> f64 {
            self.df_const
        }
    }

    fn d(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(
            year,
            Month::try_from(month).expect("Valid month (1-12)"),
            day,
        )
        .expect("Valid test date")
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
        ]
    }

    #[test]
    fn pv_with_ctx_sum_matches_direct_calculation() {
        // Test that PV aggregation with Act365F sums correctly
        let base = d(2025, 1, 1);
        let periods = quarters_2025();

        let flows = vec![
            (d(2025, 2, 15), Money::new(100.0, Currency::USD)),
            (d(2025, 5, 15), Money::new(200.0, Currency::USD)),
        ];

        let curve = FlatDiscountCurve {
            id: CurveId::new("USD-OIS"),
            base,
            df_const: 0.95,
        };

        let dc_ctx = DayCountCtx {
            frequency: Some(Frequency::quarterly()),
            calendar: None,
            bus_basis: None,
        };

        let pv_map = pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::Act365F, dc_ctx)
            .expect("PV by period calculation should succeed in test");

        // Sum of period PVs
        let sum_pv: f64 = pv_map
            .values()
            .flat_map(|m| m.values())
            .map(|m| m.amount())
            .sum();

        // Standalone NPV using default context (Act365F doesn't require special ctx)
        let total_npv = npv_static(&curve, base, DayCount::Act365F, &flows)
            .expect("NPV calculation should succeed in test");

        // Should match within tolerance
        assert!(
            (sum_pv - total_npv.amount()).abs() < 1e-10,
            "Sum of period PVs ({}) should match NPV ({})",
            sum_pv,
            total_npv.amount()
        );
    }

    #[test]
    fn pv_with_ctx_errors_on_missing_frequency_for_isma() {
        // Act/Act ISMA requires frequency in context
        let base = d(2025, 1, 1);
        let periods = quarters_2025();
        let flows = vec![(d(2025, 2, 15), Money::new(100.0, Currency::USD))];

        let curve = FlatDiscountCurve {
            id: CurveId::new("USD-OIS"),
            base,
            df_const: 0.95,
        };

        // Missing frequency for ISMA should error
        let dc_ctx = DayCountCtx {
            frequency: None, // Missing!
            calendar: None,
            bus_basis: None,
        };

        let result =
            pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::ActActIsma, dc_ctx);

        assert!(result.is_err(), "Should error when ISMA frequency missing");
    }

    #[test]
    fn pv_by_period_deterministic_multi_currency() {
        // Multi-currency PV aggregation should preserve currency separation
        let base = d(2025, 1, 1);
        let periods = quarters_2025();

        let flows = vec![
            (d(2025, 2, 15), Money::new(100.0, Currency::USD)),
            (d(2025, 2, 20), Money::new(200.0, Currency::EUR)),
            (d(2025, 5, 10), Money::new(50.0, Currency::USD)),
        ];

        let curve = FlatDiscountCurve {
            id: CurveId::new("USD-OIS"),
            base,
            df_const: 0.95,
        };

        let dc_ctx = DayCountCtx {
            frequency: Some(Frequency::quarterly()),
            calendar: None,
            bus_basis: None,
        };

        let pv_map = pv_by_period_with_ctx(&flows, &periods, &curve, base, DayCount::Act365F, dc_ctx)
            .expect("PV by period calculation should succeed in test");

        // Q1 should have both USD and EUR
        let q1 = pv_map
            .get(&PeriodId::quarter(2025, 1))
            .expect("Q1 should exist");
        assert_eq!(q1.len(), 2);
        assert!(q1.contains_key(&Currency::USD));
        assert!(q1.contains_key(&Currency::EUR));

        // Q2 should have only USD
        let q2 = pv_map
            .get(&PeriodId::quarter(2025, 2))
            .expect("Q2 should exist");
        assert_eq!(q2.len(), 1);
        assert!(q2.contains_key(&Currency::USD));
    }
}

#[cfg(test)]
mod pv_tests {
    use super::*;
    use finstack_core::market_data::traits::{Discounting, Survival, TermStructure};
    use finstack_core::types::CurveId;
    use time::Month;

    struct FlatDiscountCurve {
        id: CurveId,
        base: Date,
        df_const: f64,
    }

    impl TermStructure for FlatDiscountCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    impl Discounting for FlatDiscountCurve {
        fn base_date(&self) -> Date {
            self.base
        }
        fn df(&self, _t: f64) -> f64 {
            self.df_const
        }
    }

    struct FlatHazardCurve {
        id: CurveId,
        #[allow(dead_code)]
        base: Date,
        sp_const: f64,
    }

    impl TermStructure for FlatHazardCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    impl Survival for FlatHazardCurve {
        fn sp(&self, _t: f64) -> f64 {
            self.sp_const
        }
    }

    fn d(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(
            year,
            Month::try_from(month).expect("Valid month (1-12)"),
            day,
        )
        .expect("Valid test date")
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
        ]
    }

    #[test]
    fn pv_by_period_sum_matches_npv() {
        let base = d(2025, 1, 1);
        let periods = quarters_2025();
        let flows = vec![
            (d(2025, 2, 15), Money::new(100.0, Currency::USD)),
            (d(2025, 5, 15), Money::new(200.0, Currency::USD)),
        ];

        let curve = FlatDiscountCurve {
            id: CurveId::new("USD-OIS"),
            base,
            df_const: 0.95, // Flat 5% discount
        };

        let pv_map = pv_by_period(&flows, &periods, &curve, base, DayCount::Act365F);

        // Q1 should have PV = 100 * 0.95 = 95
        let q1_pv = pv_map
            .get(&PeriodId::quarter(2025, 1))
            .and_then(|m| m.get(&Currency::USD))
            .map(|m| m.amount())
            .unwrap_or(0.0);
        assert!((q1_pv - 95.0).abs() < 1e-10);

        // Q2 should have PV = 200 * 0.95 = 190
        let q2_pv = pv_map
            .get(&PeriodId::quarter(2025, 2))
            .and_then(|m| m.get(&Currency::USD))
            .map(|m| m.amount())
            .unwrap_or(0.0);
        assert!((q2_pv - 190.0).abs() < 1e-10);

        // Sum should equal total NPV
        use finstack_core::cashflow::discounting::npv_static;
        let total_npv = npv_static(&curve, base, DayCount::Act365F, &flows)
            .expect("NPV calculation should succeed in test");
        let sum_pv = q1_pv + q2_pv;
        assert!((sum_pv - total_npv.amount()).abs() < 1e-10);
    }

    #[test]
    fn pv_by_period_respects_boundaries() {
        let base = d(2025, 1, 1);
        let periods = quarters_2025();
        // Flow exactly on period boundary should go to next period
        let flows = vec![(d(2025, 4, 1), Money::new(100.0, Currency::USD))];

        let curve = FlatDiscountCurve {
            id: CurveId::new("USD-OIS"),
            base,
            df_const: 1.0,
        };

        let pv_map = pv_by_period(&flows, &periods, &curve, base, DayCount::Act365F);

        // Should be in Q2, not Q1
        assert!(pv_map.get(&PeriodId::quarter(2025, 1)).is_none());
        let q2_pv = pv_map
            .get(&PeriodId::quarter(2025, 2))
            .and_then(|m| m.get(&Currency::USD))
            .map(|m| m.amount())
            .unwrap_or(0.0);
        assert!((q2_pv - 100.0).abs() < 1e-10);
    }

    #[test]
    fn pv_by_period_multi_currency_separation() {
        let base = d(2025, 1, 1);
        let periods = quarters_2025();
        let flows = vec![
            (d(2025, 2, 15), Money::new(100.0, Currency::USD)),
            (d(2025, 2, 20), Money::new(200.0, Currency::EUR)),
        ];

        let curve = FlatDiscountCurve {
            id: CurveId::new("USD-OIS"),
            base,
            df_const: 0.95,
        };

        let pv_map = pv_by_period(&flows, &periods, &curve, base, DayCount::Act365F);

        let q1 = pv_map
            .get(&PeriodId::quarter(2025, 1))
            .expect("Q1 should exist");
        assert_eq!(q1.len(), 2); // Both currencies present
        assert!((q1[&Currency::USD].amount() - 95.0).abs() < 1e-10);
        assert!((q1[&Currency::EUR].amount() - 190.0).abs() < 1e-10);
    }

    #[test]
    fn test_pv_by_period_credit_adjusted() {
        let base = d(2025, 1, 1);
        let periods = quarters_2025();
        let flows = vec![(d(2025, 2, 15), Money::new(100.0, Currency::USD))];

        let disc_curve = FlatDiscountCurve {
            id: CurveId::new("USD-OIS"),
            base,
            df_const: 0.95,
        };

        let hazard_curve = FlatHazardCurve {
            id: CurveId::new("AAPL-HAZARD"),
            base,
            sp_const: 0.90, // 90% survival probability
        };

        let pv_map = pv_by_period_credit_adjusted(
            &flows,
            &periods,
            &disc_curve,
            Some(&hazard_curve),
            base,
            DayCount::Act365F,
        );

        // PV should be 100 * 0.95 * 0.90 = 85.5
        let q1_pv = pv_map
            .get(&PeriodId::quarter(2025, 1))
            .and_then(|m| m.get(&Currency::USD))
            .map(|m| m.amount())
            .unwrap_or(0.0);
        assert!((q1_pv - 85.5).abs() < 1e-10);
    }
}
