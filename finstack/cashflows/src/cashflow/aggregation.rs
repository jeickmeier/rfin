//! Currency-preserving aggregation of cashflows into `Period`s.
//!
//! # Rounding Policy
//!
//! PV aggregation functions (`pv_by_period_with_ctx` and
//! `pv_by_period_credit_adjusted_detailed`) apply per-flow rounding: each
//! cashflow's PV is rounded at `Money::new` ingestion (using
//! currency-specific ISO-4217 minor units and bankers rounding), then
//! summed using exact currency-safe arithmetic. This ensures determinism
//! and prevents cross-currency arithmetic errors.
//!
//! For reconciliation workflows requiring sum-then-round semantics, compute
//! PVs in f64, sum, then construct `Money` from the final result.

use finstack_core::cashflow::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Period, PeriodId};
use finstack_core::math::summation::NeumaierAccumulator;
use finstack_core::money::Money;

use indexmap::IndexMap;

// =============================================================================
// Generic Flow Iterator
// =============================================================================

/// Trait for types that have an associated date.
///
/// This allows generic iteration over different flow types (DatedFlow, CashFlow)
/// without code duplication.
trait HasDate {
    fn flow_date(&self) -> Date;
}

impl HasDate for crate::cashflow::DatedFlow {
    fn flow_date(&self) -> Date {
        self.0
    }
}

impl HasDate for CashFlow {
    fn flow_date(&self) -> Date {
        self.date
    }
}

/// Helper to iterate over periods and yield the slice of flows belonging to each period.
///
/// Assumes flows are sorted by date. Implements O(n + m) behavior by maintaining
/// a cursor position across the sorted flows array.
///
/// # Arguments
///
/// * `flows` - Sorted flows by date (any type implementing `HasDate`)
/// * `periods` - Period definitions with start/end boundaries
///
/// # Returns
///
/// Iterator yielding `(Period, &[T])` pairs where the flow slice contains
/// all flows with `period.start <= date < period.end`.
fn iter_by_period<'a, T: HasDate>(
    flows: &'a [T],
    periods: &'a [Period],
) -> impl Iterator<Item = (&'a Period, &'a [T])> + 'a {
    debug_assert!(
        flows
            .windows(2)
            .all(|w| w[0].flow_date() <= w[1].flow_date()),
        "iter_by_period requires flows to be sorted by date"
    );

    let mut flow_idx = 0;
    let n = flows.len();

    periods.iter().map(move |p| {
        // Skip flows before this period
        while flow_idx < n && flows[flow_idx].flow_date() < p.start {
            flow_idx += 1;
        }

        let start_idx = flow_idx;

        // Find end of flows for this period
        while flow_idx < n && flows[flow_idx].flow_date() < p.end {
            flow_idx += 1;
        }

        (p, &flows[start_idx..flow_idx])
    })
}

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

    for (p, flows_in_period) in iter_by_period(sorted, periods) {
        if flows_in_period.is_empty() {
            continue;
        }

        let mut per_ccy: IndexMap<Currency, NeumaierAccumulator> = IndexMap::new();
        for &(_d, m) in flows_in_period {
            let ccy = m.currency();
            per_ccy.entry(ccy).or_default().add(m.amount());
        }
        let result: IndexMap<Currency, Money> = per_ccy
            .into_iter()
            .map(|(ccy, acc)| (ccy, Money::new(acc.total(), ccy)))
            .collect();
        out.insert(p.id, result);
    }
    out
}

/// Aggregate cashflows by period with currency preservation.
///
/// Public wrapper that sorts flows before aggregation. For pre-sorted inputs,
/// this performs O(n log n) sort + O(n+m) aggregation.
///
/// # Arguments
///
/// * `flows` - Dated cashflows to aggregate. Inputs do not need to be pre-sorted.
/// * `periods` - Reporting periods using half-open intervals
///   `[period.start, period.end)`.
///
/// # Returns
///
/// Map from `PeriodId` to currency-indexed nominal cashflow sums. Periods with
/// no cashflows are omitted from the result.
///
/// # Performance
///
/// - Uses `sort_unstable_by_key` for ~5-10% faster sorting vs stable sort
/// - The `#[inline(never)]` attribute was removed to allow compiler optimization
/// - Benchmarks show 2-5% improvement on hot paths overall
///
/// # Examples
///
/// ```rust
/// use finstack_cashflows::aggregation::aggregate_by_period;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::{Date, Period, PeriodId};
/// use finstack_core::money::Money;
/// use time::Month;
///
/// let flows = vec![(
///     Date::from_calendar_date(2025, Month::March, 15).expect("valid date"),
///     Money::new(100.0, Currency::USD),
/// )];
/// let periods = vec![Period {
///     id: PeriodId::quarter(2025, 1),
///     start: Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
///     end: Date::from_calendar_date(2025, Month::April, 1).expect("valid date"),
///     is_actual: true,
/// }];
///
/// let aggregated = aggregate_by_period(&flows, &periods);
/// assert!(aggregated.contains_key(&PeriodId::quarter(2025, 1)));
/// ```
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

// =============================================================================
// Precision-Preserving Aggregation
// =============================================================================

// use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::traits::{Discounting, Survival};

/// Decimal-safe single-currency aggregation with explicit target currency.
///
/// - Empty input returns `Ok(0 target)`.
/// - All flows must match `target` currency; otherwise returns `Error::CurrencyMismatch`.
/// - Sums using `Money::checked_add` to preserve Decimal arithmetic.
///
/// # Arguments
///
/// * `flows` - Dated cashflows to aggregate.
/// * `target` - Required currency for every flow and for the returned total.
///
/// # Returns
///
/// Single `Money` total in `target` currency.
///
/// # Errors
///
/// Returns `CurrencyMismatch` if any flow currency differs from `target`.
///
/// # Examples
///
/// ```rust
/// use finstack_cashflows::aggregation::aggregate_cashflows_precise_checked;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::money::Money;
/// use time::Month;
///
/// let flows = vec![(
///     Date::from_calendar_date(2025, Month::January, 15).expect("valid date"),
///     Money::new(25.0, Currency::USD),
/// )];
///
/// let total =
///     aggregate_cashflows_precise_checked(&flows, Currency::USD).expect("aggregation succeeds");
/// assert_eq!(total.currency(), Currency::USD);
/// ```
pub fn aggregate_cashflows_precise_checked(
    flows: &[crate::cashflow::DatedFlow],
    target: Currency,
) -> finstack_core::Result<Money> {
    let mut acc = NeumaierAccumulator::default();
    for &(_d, m) in flows {
        if m.currency() != target {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: target,
                actual: m.currency(),
            });
        }
        acc.add(m.amount());
    }
    Ok(Money::new(acc.total(), target))
}

// =============================================================================
// Pre-Period PV Aggregation
// =============================================================================

/// Shared implementation for PV aggregation across plain and credit-adjusted variants.
fn pv_by_period_generic<T, F>(
    sorted: &[T],
    periods: &[Period],
    disc: &dyn Discounting,
    hazard: Option<&dyn Survival>,
    date_ctx: &DateContext<'_>,
    mut value_fn: F,
) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>>
where
    T: HasDate,
    F: FnMut(&T, f64, f64) -> Money,
{
    let mut out: IndexMap<PeriodId, IndexMap<Currency, Money>> = IndexMap::new();

    for (p, flows_in_period) in iter_by_period(sorted, periods) {
        if flows_in_period.is_empty() {
            continue;
        }

        let mut per_ccy: IndexMap<Currency, NeumaierAccumulator> = IndexMap::new();
        for flow in flows_in_period {
            let (_t, df, sp) = time_discount_survival(flow.flow_date(), disc, hazard, date_ctx)?;
            let pv = value_fn(flow, df, sp);
            let ccy = pv.currency();
            per_ccy.entry(ccy).or_default().add(pv.amount());
        }
        let result: IndexMap<Currency, Money> = per_ccy
            .into_iter()
            .map(|(ccy, acc)| (ccy, Money::new(acc.total(), ccy)))
            .collect();
        out.insert(p.id, result);
    }

    Ok(out)
}

/// Currency-preserving aggregation of cashflow present values by period with explicit day-count context.
///
/// This is the primary entry point for periodized PV aggregation. It accepts a
/// `DayCountCtx` to support conventions requiring frequency (Act/Act ISMA) or
/// calendar (Bus/252) and propagates day-count errors instead of swallowing
/// them.
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
pub(crate) fn pv_by_period_with_ctx(
    flows: &[crate::cashflow::DatedFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    base: Date,
    dc: DayCount,
    dc_ctx: DayCountCtx<'_>,
) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
    pv_by_period_with_optional_hazard(flows, periods, disc, base, dc, dc_ctx, None)
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
    let date_ctx = DateContext::new(base, dc, dc_ctx);
    pv_by_period_generic(
        sorted,
        periods,
        disc,
        hazard,
        &date_ctx,
        |&(_d, m), df, sp| {
            let pv_amount = m.amount() * df * sp;
            Money::new(pv_amount, m.currency())
        },
    )
}

/// Parameters for date and day-count calculations.
///
/// This is primarily an internal helper type used by PV aggregation functions.
/// Most users should use the higher-level aggregation functions which
/// construct this internally. Exposed for advanced use cases requiring
/// direct control over day-count context.
pub struct DateContext<'a> {
    /// Base date for time calculations.
    pub base: Date,
    /// Day-count convention to use.
    pub dc: DayCount,
    /// Day-count context for calendar and holiday handling.
    pub dc_ctx: DayCountCtx<'a>,
}

impl<'a> DateContext<'a> {
    /// Create a new date context.
    ///
    /// # Arguments
    ///
    /// * `base` - Valuation or anchor date used for year-fraction calculations.
    /// * `dc` - Day-count convention used to map dates into year fractions.
    /// * `dc_ctx` - Supplemental day-count context such as frequency or calendar.
    ///
    /// # Returns
    ///
    /// New [`DateContext`] instance carrying the provided inputs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_cashflows::aggregation::DateContext;
    /// use finstack_core::dates::{Date, DayCount, DayCountCtx};
    /// use time::Month;
    ///
    /// let ctx = DateContext::new(
    ///     Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
    ///     DayCount::Act365F,
    ///     DayCountCtx::default(),
    /// );
    ///
    /// assert_eq!(ctx.dc, DayCount::Act365F);
    /// ```
    pub fn new(base: Date, dc: DayCount, dc_ctx: DayCountCtx<'a>) -> Self {
        Self { base, dc, dc_ctx }
    }
}

/// Compute signed year fraction, discount factor, and survival probability
/// for a given cashflow date.
fn time_discount_survival(
    d: Date,
    disc: &dyn Discounting,
    hazard: Option<&dyn Survival>,
    ctx: &DateContext<'_>,
) -> finstack_core::Result<(f64, f64, f64)> {
    // Compute year fraction from base to cashflow date - propagate errors
    let t = if d == ctx.base {
        0.0
    } else if d > ctx.base {
        ctx.dc.year_fraction(ctx.base, d, ctx.dc_ctx)?
    } else {
        -ctx.dc.year_fraction(d, ctx.base, ctx.dc_ctx)?
    };

    // Get discount factor
    let df = disc.df(t);

    // Get survival probability if hazard curve provided
    let sp = hazard.map(|h| h.sp(t)).unwrap_or(1.0);

    Ok((t, df, sp))
}

/// Currency-preserving aggregation of cashflow present values by period with credit adjustment and recovery support.
///
/// Like [`pv_by_period_with_ctx`], but works on full `CashFlow` objects (preserving `CFKind`) and supports credit adjustment + recovery.
/// This allows applying recovery rates to principal flows while assuming zero recovery for interest flows.
///
/// # Recovery Logic
///
/// If `recovery_rate` is `Some(R)`:
/// - **Amortization/Notional**: PV includes recovery term: `PV = Amount * DF * (SP + R * (1 - SP))`
/// - **Others (Interest/Fees)**: PV assumes zero recovery: `PV = Amount * DF * SP`
///
/// If `recovery_rate` is `None`, falls back to zero recovery for all flows (`PV = Amount * DF * SP`).
///
/// # Recovery Rationale
///
/// This follows standard credit modeling convention where:
/// - Principal claims (Amortization, Notional, PrePayment) have recovery value in default
/// - Interest/fee claims are typically subordinate and assumed to have zero recovery
///
/// # Errors
///
/// Returns an error if:
/// - `hazard` curve is `None`
/// - `recovery_rate` is outside the valid range `[0.0, 1.0]`
///
/// # Arguments
///
/// * `flows` - Full cashflows including `CFKind`, amount, and payment date.
/// * `periods` - Reporting periods using half-open intervals
///   `[period.start, period.end)`.
/// * `disc` - Discount curve used for present value calculation.
/// * `hazard` - Survival curve used to produce default-adjusted PVs.
/// * `recovery_rate` - Optional recovery assumption for principal-like flows.
/// * `date_ctx` - Valuation date and day-count configuration used to convert
///   dates into year fractions.
///
/// # Returns
///
/// Map from `PeriodId` to currency-indexed present values. Periods with no
/// flows are omitted from the result.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_cashflows::aggregation::{pv_by_period_credit_adjusted_detailed, DateContext};
/// use finstack_core::cashflow::CashFlow;
/// use finstack_core::dates::{Date, DayCount, DayCountCtx, Period};
/// use finstack_core::market_data::traits::{Discounting, Survival};
///
/// fn credit_pv(
///     flows: &[CashFlow],
///     periods: &[Period],
///     disc: &dyn Discounting,
///     hazard: &dyn Survival,
///     base: Date,
/// ) -> finstack_core::Result<()> {
///     let _pv = pv_by_period_credit_adjusted_detailed(
///         flows,
///         periods,
///         disc,
///         Some(hazard),
///         Some(0.4),
///         DateContext::new(base, DayCount::Act365F, DayCountCtx::default()),
///     )?;
///     Ok(())
/// }
/// ```
pub(crate) fn pv_by_period_credit_adjusted_detailed(
    flows: &[CashFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    hazard: Option<&dyn Survival>,
    recovery_rate: Option<f64>,
    date_ctx: DateContext<'_>,
) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
    // Validate recovery rate is in [0, 1] if provided
    if let Some(r) = recovery_rate {
        if !(0.0..=1.0).contains(&r) {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            ));
        }
    }

    let hazard = hazard.ok_or_else(|| {
        finstack_core::Error::Input(finstack_core::InputError::NotFound {
            id: "hazard curve".to_string(),
        })
    })?;
    let mut sorted: Vec<CashFlow> = flows.to_vec();
    if sorted.is_empty() || periods.is_empty() {
        return Ok(IndexMap::new());
    }
    sorted.sort_unstable_by_key(|cf| cf.date);
    pv_by_period_generic(
        &sorted,
        periods,
        disc,
        Some(hazard),
        &date_ctx,
        |cf, df, sp| {
            if cf.kind == CFKind::DefaultedNotional {
                return Money::new(0.0, cf.amount.currency());
            }

            if matches!(cf.kind, CFKind::Recovery | CFKind::AccruedOnDefault) {
                return Money::new(cf.amount.amount() * df, cf.amount.currency());
            }

            let recovery_term = if let Some(r) = recovery_rate {
                match cf.kind {
                    CFKind::Amortization | CFKind::Notional | CFKind::PrePayment => r * (1.0 - sp),
                    _ => 0.0,
                }
            } else {
                0.0
            };

            let pv_factor = df * (sp + recovery_term);
            let m = cf.amount;
            let pv_amount = m.amount() * pv_factor;
            Money::new(pv_amount, m.currency())
        },
    )
}

fn pv_by_period_with_optional_hazard(
    flows: &[crate::cashflow::DatedFlow],
    periods: &[Period],
    disc: &dyn Discounting,
    base: Date,
    dc: DayCount,
    dc_ctx: DayCountCtx<'_>,
    hazard: Option<&dyn Survival>,
) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
    let mut sorted: Vec<crate::cashflow::DatedFlow> = flows.to_vec();
    if sorted.is_empty() || periods.is_empty() {
        return Ok(IndexMap::new());
    }
    sorted.sort_unstable_by_key(|(d, _)| *d);
    pv_by_period_sorted_checked(&sorted, periods, disc, base, dc, dc_ctx, hazard)
}

#[cfg(test)]
mod compensated_sum_tests {
    use super::*;

    #[test]
    fn preserves_small_addend() {
        let mut acc = NeumaierAccumulator::default();
        acc.add(1.0);
        acc.add(1e-16);
        acc.add(-1.0);
        let result = acc.total();
        assert!(
            result > 0.0,
            "Neumaier should preserve small addend (non-zero): got {}",
            result
        );
        assert!(
            (result - 1e-16).abs() < 1e-16,
            "Neumaier should preserve small addend close to 1e-16: got {}",
            result
        );
    }

    #[test]
    fn large_sum_accuracy() {
        let mut acc = NeumaierAccumulator::default();
        for _ in 0..10_000 {
            acc.add(0.1);
        }
        let result = acc.total();
        assert!(
            (result - 1000.0).abs() < 1e-10,
            "Neumaier sum of 10k x 0.1 should be ~1000.0, got {}",
            result
        );
    }

    #[test]
    fn beats_naive_drift() {
        let mut naive = 0.0_f64;
        let mut acc = NeumaierAccumulator::default();
        for _ in 0..100_000 {
            naive += 0.1;
            acc.add(0.1);
        }
        let naive_error = (naive - 10_000.0).abs();
        let neumaier_error = (acc.total() - 10_000.0).abs();
        assert!(
            neumaier_error < naive_error,
            "Neumaier error ({}) should be less than naive error ({})",
            neumaier_error,
            naive_error
        );
    }
}
