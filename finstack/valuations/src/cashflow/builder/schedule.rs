//! Schedule generation from the builder state.
//!
//! Provides the canonical `CashFlowSchedule` type and helpers for sorting and
//! deriving schedule metadata. Downstream pricing/risk code consumes this shape.

use crate::cashflow::aggregation::{pv_by_period, pv_by_period_credit_adjusted};
use crate::cashflow::primitives::Notional;
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::{
    traits::{Discounting, Survival},
    MarketContext,
};
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;
use indexmap::IndexMap;
use std::sync::Arc;

use super::types::{FixedCouponSpec, FloatingCouponSpec};

pub(crate) fn kind_rank(kind: CFKind) -> u8 {
    match kind {
        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
        CFKind::Fee => 1,
        CFKind::Amortization => 2,
        CFKind::PIK => 3,
        CFKind::Notional => 4,
        _ => 5,
    }
}

pub(crate) fn finalize_flows(
    mut flows: Vec<CashFlow>,
    fixed: &[FixedCouponSpec],
    floating: &[FloatingCouponSpec],
) -> (Vec<CashFlow>, CashflowMeta, DayCount) {
    flows.sort_by(|a, b| {
        use core::cmp::Ordering;
        match a.date.cmp(&b.date) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => kind_rank(a.kind).cmp(&kind_rank(b.kind)),
        }
    });

    let mut cals: Vec<String> = Vec::new();
    for s in fixed {
        if let Some(id) = &s.calendar_id {
            cals.push(id.clone());
        }
    }
    for s in floating {
        if let Some(id) = &s.calendar_id {
            cals.push(id.clone());
        }
    }
    cals.sort_unstable();
    cals.dedup();
    let meta = CashflowMeta { calendar_ids: cals };

    let out_dc = if let Some(s) = fixed.first() {
        s.dc
    } else if let Some(s) = floating.first() {
        s.dc
    } else {
        DayCount::Act365F
    };
    (flows, meta, out_dc)
}

/// Minimal schedule metadata for a built schedule.
///
/// Tracks referenced calendar IDs so callers can understand adjustment context.
#[derive(Debug, Clone, Default)]
pub struct CashflowMeta {
    pub calendar_ids: Vec<String>,
}

/// Cashflow schedule output from the composable builder.
///
/// Contains ordered cashflows plus notional and a representative `DayCount`.
/// Methods provide convenient accessors commonly used by pricing and analysis.
#[derive(Debug, Clone)]
pub struct CashFlowSchedule {
    pub flows: Vec<CashFlow>,
    pub notional: Notional,
    pub day_count: DayCount,
    pub meta: CashflowMeta,
}

/// Options for period-aligned DataFrame exports.
#[derive(Debug, Clone, Default)]
pub struct PeriodDataFrameOptions<'a> {
    pub hazard_curve_id: Option<&'a str>,
    pub forward_curve_id: Option<&'a str>,
    pub as_of: Option<Date>,
    pub day_count: Option<DayCount>,
    pub facility_limit: Option<Money>,
    pub include_floating_decomposition: bool,
}

impl CashFlowSchedule {
    /// Create a new cashflow builder (standard Rust pattern).
    ///
    /// This is the recommended entry point for building cashflow schedules.
    /// Returns a `CashflowBuilder` that can be configured and built.
    ///
    /// # Example
    /// ```ignore
    /// let schedule = CashFlowSchedule::builder()
    ///     .principal(notional, issue, maturity)
    ///     .fixed_cf(spec)
    ///     .build()?;
    /// ```
    pub fn builder() -> super::CashflowBuilder {
        super::CashflowBuilder::default()
    }

    /// Returns the list of dates for all flows in schedule order.
    pub fn dates(&self) -> Vec<Date> {
        self.flows.iter().map(|cf| cf.date).collect()
    }

    /// Returns an iterator over flows of the given `CFKind`.
    pub fn flows_of_kind(&self, kind: CFKind) -> impl Iterator<Item = &CashFlow> {
        self.flows.iter().filter(move |cf| cf.kind == kind)
    }

    /// Outstanding principal path computed from principal/PIK/amortization flows.
    ///
    /// Note: Amortization amounts in the schedule are stored as POSITIVE values
    /// (the builder internally manages the reduction of outstanding balance).
    /// PIK amounts are positive and increase outstanding.
    ///
    /// Example
    /// -------
    /// ```rust
    /// use finstack_core::dates::Date;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_valuations::cashflow::builder::schedule::{CashFlowSchedule, CashflowMeta};
    /// use finstack_core::cashflow::primitives::{CashFlow, CFKind, Notional};
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let notional = Notional { initial: Money::new(100.0, Currency::USD), amort: Default::default() };
    /// let flows = vec![
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(10.0, Currency::USD), kind: CFKind::Amortization, accrual_factor: 0.0 },
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(5.0, Currency::USD), kind: CFKind::PIK, accrual_factor: 0.0 },
    /// ];
    /// let s = CashFlowSchedule { flows, notional, day_count: finstack_core::dates::DayCount::Act365F, meta: CashflowMeta::default() };
    /// let path = s.outstanding_path();
    /// assert_eq!(path.len(), 2);
    /// assert_eq!(path[0].1.amount(), 90.0);  // 100 - 10 = 90
    /// assert_eq!(path[1].1.amount(), 95.0);  // 90 + 5 = 95
    /// ```
    pub fn outstanding_path(&self) -> Vec<(Date, Money)> {
        let mut out = Vec::new();
        let mut outstanding = self.notional.initial;
        for cf in &self.flows {
            match cf.kind {
                CFKind::Amortization => {
                    // Amortization amounts are stored as positive in the builder
                    // but economically represent principal reductions
                    outstanding = outstanding.checked_sub(cf.amount).unwrap();
                }
                CFKind::PIK => {
                    outstanding = outstanding.checked_add(cf.amount).unwrap();
                }
                _ => {}
            }
            out.push((cf.date, outstanding));
        }
        out
    }

    // Convenience iterators for callers to avoid ad-hoc filtering.
    pub fn coupons(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub)
    }

    pub fn amortizations(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Amortization)
    }

    pub fn redemptions(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Notional && cf.amount.amount() > 0.0)
    }

    /// End-of-date outstanding path: one entry per unique date after applying
    /// Amortization/PIK on that date. Redemption does not reduce outstanding here.
    ///
    /// Note: Amortization amounts in the schedule are stored as POSITIVE values.
    pub fn outstanding_by_date(&self) -> Vec<(Date, Money)> {
        let mut result: Vec<(Date, Money)> = Vec::new();
        if self.flows.is_empty() {
            return result;
        }

        let mut outstanding = self.notional.initial;

        let mut i = 0usize;
        while i < self.flows.len() {
            let d = self.flows[i].date;
            // Process all flows on this date in their deterministic order
            let mut j = i;
            while j < self.flows.len() && self.flows[j].date == d {
                match self.flows[j].kind {
                    CFKind::Amortization => {
                        outstanding = outstanding.checked_sub(self.flows[j].amount).unwrap();
                    }
                    CFKind::PIK => {
                        outstanding = outstanding.checked_add(self.flows[j].amount).unwrap();
                    }
                    _ => {}
                }
                j += 1;
            }
            result.push((d, outstanding));
            i = j;
        }

        result
    }

    /// Compute pre-period present values aggregated by period.
    ///
    /// Groups cashflows by period and computes the present value of each cashflow
    /// discounted back to the base date. Returns a map from `PeriodId` to currency-indexed
    /// PV sums for that period.
    ///
    /// # Arguments
    /// * `periods` - Period definitions with start/end boundaries
    /// * `disc` - Discount curve for present value calculation
    /// * `base` - Base date for discounting (typically valuation date)
    /// * `dc` - Day count convention for year fraction calculation
    ///
    /// # Returns
    /// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows
    /// are omitted from the result.
    pub fn pre_period_pv(
        &self,
        periods: &[Period],
        disc: &dyn Discounting,
        base: Date,
        dc: DayCount,
    ) -> IndexMap<PeriodId, IndexMap<Currency, Money>> {
        let flows: Vec<(Date, Money)> = self.flows.iter().map(|cf| (cf.date, cf.amount)).collect();
        pv_by_period(&flows, periods, disc, base, dc)
    }

    /// Compute pre-period present values with market context support for credit adjustment.
    ///
    /// Similar to `pre_period_pv`, but uses `MarketContext` to look up curves by ID,
    /// enabling credit-adjusted discounting via hazard curves.
    ///
    /// # Arguments
    /// * `periods` - Period definitions with start/end boundaries
    /// * `market` - Market context containing discount and optional hazard curves
    /// * `disc_curve_id` - Identifier for the discount curve in the market context
    /// * `hazard_curve_id` - Optional identifier for hazard curve (credit adjustment)
    /// * `base` - Base date for discounting (typically valuation date)
    /// * `dc` - Day count convention for year fraction calculation
    ///
    /// # Returns
    /// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows
    /// are omitted from the result.
    ///
    /// # Errors
    /// Returns an error if the discount curve is not found, or if hazard_curve_id
    /// is provided but the curve is not found in the market context.
    pub fn pre_period_pv_with_market(
        &self,
        periods: &[Period],
        market: &MarketContext,
        disc_curve_id: &CurveId,
        hazard_curve_id: Option<&CurveId>,
        base: Date,
        dc: DayCount,
    ) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
        let flows: Vec<(Date, Money)> = self.flows.iter().map(|cf| (cf.date, cf.amount)).collect();

        // Get discount curve
        let disc_arc = market.get_discount(disc_curve_id.as_str())?;
        let disc: &dyn Discounting = disc_arc.as_ref();

        // Get hazard curve if provided
        // Note: We need to store the Arc to keep the reference alive for the function scope
        let hazard_arc_opt: Option<Arc<HazardCurve>> = if let Some(hazard_id) = hazard_curve_id {
            Some(market.get_hazard(hazard_id.as_str())?)
        } else {
            None
        };

        let hazard: Option<&dyn Survival> = hazard_arc_opt
            .as_ref()
            .map(|arc| arc.as_ref() as &dyn Survival);

        Ok(pv_by_period_credit_adjusted(
            &flows, periods, disc, hazard, base, dc,
        ))
    }

    /// Period-aligned DataFrame-like export with optional credit and floating decomposition.
    ///
    /// This computes all derived columns (discount factors, survival probabilities,
    /// base rate, spread, all-in rate, unfunded amounts) in Rust for consistency
    /// across language bindings. Bindings should only perform type conversion.
    /// * `options` - Additional configuration (hazard/forward IDs, overrides, facility limits).
    pub fn to_period_dataframe(
        &self,
        periods: &[finstack_core::dates::Period],
        market: &MarketContext,
        discount_curve_id: &str,
        options: PeriodDataFrameOptions<'_>,
    ) -> finstack_core::Result<PeriodDataFrame> {
        use finstack_core::dates::DayCountCtx;
        let dc = options.day_count.unwrap_or(self.day_count);

        let disc_arc = market.get_discount(discount_curve_id)?;
        let base = options.as_of.unwrap_or_else(|| disc_arc.base_date());

        let has_hazard = options.hazard_curve_id.is_some();
        let hazard_arc_opt = if let Some(hz) = options.hazard_curve_id {
            Some(market.get_hazard(hz)?)
        } else {
            None
        };
        let forward_arc_opt = if options.include_floating_decomposition {
            options
                .forward_curve_id
                .and_then(|fid| market.get_forward(fid).ok())
        } else {
            None
        };

        let facility_limit = options.facility_limit;

        // Columns
        let mut start_dates: Vec<Date> = Vec::new();
        let mut end_dates: Vec<Date> = Vec::new();
        let mut pay_dates: Vec<Date> = Vec::new();
        let mut cf_types: Vec<CFKind> = Vec::new();
        let mut currencies: Vec<Currency> = Vec::new();
        let mut notionals: Vec<Option<f64>> = Vec::new();
        let mut yr_fraqs: Vec<f64> = Vec::new();
        let mut days: Vec<i64> = Vec::new();
        let mut amounts: Vec<f64> = Vec::new();
        let mut discount_factors: Vec<f64> = Vec::new();
        let mut survival_probs: Option<Vec<Option<f64>>> =
            if has_hazard { Some(Vec::new()) } else { None };
        let mut pvs: Vec<f64> = Vec::new();
        let mut unfunded_amounts: Option<Vec<Option<f64>>> =
            facility_limit.as_ref().map(|_| Vec::new());
        let mut commitment_amounts: Option<Vec<Option<f64>>> =
            facility_limit.as_ref().map(|_| Vec::new());
        let mut base_rates: Option<Vec<Option<f64>>> = if options.include_floating_decomposition {
            Some(Vec::new())
        } else {
            None
        };
        let mut spreads: Option<Vec<Option<f64>>> = if options.include_floating_decomposition {
            Some(Vec::new())
        } else {
            None
        };
        let mut allin_rates: Vec<Option<f64>> = Vec::new();

        // Track outstanding drawn balance for Notional column
        let mut outstanding = self.notional.initial;

        for cf in &self.flows {
            // Find containing period (inclusive end)
            let period_opt = periods
                .iter()
                .find(|p| cf.date >= p.start && cf.date <= p.end);
            if period_opt.is_none() {
                continue;
            }
            let period = period_opt.unwrap();

            // Outstanding before this cashflow
            let outstanding_pre = outstanding;
            match cf.kind {
                CFKind::Amortization => {
                    outstanding = outstanding.checked_sub(cf.amount)?;
                }
                CFKind::PIK => {
                    outstanding = outstanding.checked_add(cf.amount)?;
                }
                CFKind::Notional => {
                    // Draws are negative, repays are positive from lender perspective
                    outstanding = outstanding.checked_sub(cf.amount)?;
                }
                _ => {}
            }

            // Basic columns
            start_dates.push(period.start);
            end_dates.push(period.end);
            pay_dates.push(cf.date);
            cf_types.push(cf.kind);
            currencies.push(cf.amount.currency());
            amounts.push(cf.amount.amount());

            // Notional for interest-like rows
            let notional_val =
                if matches!(cf.kind, CFKind::Fixed | CFKind::Stub | CFKind::FloatReset)
                    || cf.accrual_factor > 0.0
                {
                    Some(outstanding_pre.amount())
                } else {
                    None
                };
            notionals.push(notional_val);

            // YrFraq and Days
            let yr_fraq = dc
                .year_fraction(period.start, cf.date, DayCountCtx::default())
                .unwrap_or(0.0);
            yr_fraqs.push(yr_fraq);
            days.push((cf.date - period.start).whole_days());

            // Discount factor using schedule dc for consistency with legacy outputs
            let t = if cf.date == base {
                0.0
            } else if cf.date > base {
                dc.year_fraction(base, cf.date, DayCountCtx::default())
                    .unwrap_or(0.0)
            } else {
                -dc.year_fraction(cf.date, base, DayCountCtx::default())
                    .unwrap_or(0.0)
            };
            let df = disc_arc.df(t);
            discount_factors.push(df);

            // Survival probability
            if let (Some(h), Some(spv)) = (hazard_arc_opt.as_ref(), survival_probs.as_mut()) {
                spv.push(Some(h.sp(t)));
            }

            // PV
            let sp_mult = if let Some(ref spv) = survival_probs {
                spv.last().copied().flatten().unwrap_or(1.0)
            } else {
                1.0
            };
            pvs.push(cf.amount.amount() * df * sp_mult);

            // Unfunded and commitment amounts
            if let Some(limit) = facility_limit.as_ref() {
                if let Some(ref mut unfunded_vec) = unfunded_amounts {
                    if limit.currency() == cf.amount.currency() {
                        let val = (limit.amount() - outstanding_pre.amount()).max(0.0);
                        unfunded_vec.push(Some(val));
                    } else {
                        unfunded_vec.push(None);
                    }
                }
                if let Some(ref mut commit_vec) = commitment_amounts {
                    if limit.currency() == cf.amount.currency() {
                        commit_vec.push(Some(limit.amount()));
                    } else {
                        commit_vec.push(None);
                    }
                }
            }

            // Floating decomposition
            let mut base_rate_opt: Option<f64> = None;
            let mut spread_opt: Option<f64> = None;
            if options.include_floating_decomposition && matches!(cf.kind, CFKind::FloatReset) {
                if let Some(ref fwd) = forward_arc_opt {
                    let reset_t = if let Some(reset_date) = cf.reset_date {
                        if reset_date == base {
                            0.0
                        } else if reset_date > base {
                            fwd.day_count()
                                .year_fraction(base, reset_date, DayCountCtx::default())
                                .unwrap_or(0.0)
                        } else {
                            -fwd.day_count()
                                .year_fraction(reset_date, base, DayCountCtx::default())
                                .unwrap_or(0.0)
                        }
                    } else {
                        fwd.day_count()
                            .year_fraction(base, period.start, DayCountCtx::default())
                            .unwrap_or(0.0)
                    };
                    let b = fwd.rate(reset_t);
                    base_rate_opt = Some(b);
                    if let (Some(not), true) = (notional_val, yr_fraq > 0.0) {
                        let eff = cf.amount.amount() / (not * yr_fraq);
                        spread_opt = Some(eff - b);
                    }
                }
            }
            if let Some(ref mut br) = base_rates {
                br.push(base_rate_opt);
            }
            if let Some(ref mut sp) = spreads {
                sp.push(spread_opt);
            }

            // All-in rate from amounts when notional and yr_fraq available
            let allin = if let (Some(not), true) = (notional_val, yr_fraq > 0.0) {
                Some(cf.amount.amount() / (not * yr_fraq))
            } else {
                None
            };
            allin_rates.push(allin);
        }

        Ok(PeriodDataFrame {
            start_dates,
            end_dates,
            pay_dates,
            cf_types,
            currencies,
            notionals,
            yr_fraqs,
            days,
            amounts,
            discount_factors,
            survival_probs,
            pvs,
            unfunded_amounts,
            commitment_amounts,
            base_rates,
            spreads,
            allin_rates,
        })
    }
}

/// Period-aligned DataFrame-like result.
#[derive(Clone)]
pub struct PeriodDataFrame {
    pub start_dates: Vec<Date>,
    pub end_dates: Vec<Date>,
    pub pay_dates: Vec<Date>,
    pub cf_types: Vec<CFKind>,
    pub currencies: Vec<Currency>,
    pub notionals: Vec<Option<f64>>,
    pub yr_fraqs: Vec<f64>,
    pub days: Vec<i64>,
    pub amounts: Vec<f64>,
    pub discount_factors: Vec<f64>,
    pub survival_probs: Option<Vec<Option<f64>>>,
    pub pvs: Vec<f64>,
    pub unfunded_amounts: Option<Vec<Option<f64>>>,
    pub commitment_amounts: Option<Vec<Option<f64>>>,
    pub base_rates: Option<Vec<Option<f64>>>,
    pub spreads: Option<Vec<Option<f64>>>,
    pub allin_rates: Vec<Option<f64>>,
}
