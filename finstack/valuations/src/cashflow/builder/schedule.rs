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

use super::specs::{FixedCouponSpec, FloatingCouponSpec};

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

    let mut cals: Vec<String> = fixed
        .iter()
        .filter_map(|s| s.calendar_id.clone())
        .chain(
            floating
                .iter()
                .filter_map(|s| s.rate_spec.calendar_id.clone()),
        )
        .collect();
    cals.sort_unstable();
    cals.dedup();
    let meta = CashflowMeta {
        calendar_ids: cals,
        facility_limit: None,
    };

    let out_dc = if let Some(s) = fixed.first() {
        s.dc
    } else if let Some(s) = floating.first() {
        s.rate_spec.dc
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
    /// Optional facility limit/commitment for instruments like RCFs
    pub facility_limit: Option<Money>,
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
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(10.0, Currency::USD), kind: CFKind::Amortization, accrual_factor: 0.0, rate: None },
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(5.0, Currency::USD), kind: CFKind::PIK, accrual_factor: 0.0, rate: None },
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

    /// End-of-date outstanding path including Notional draws/repays.
    ///
    /// Applies Amortization (reduces), PIK (increases), and Notional
    /// (draws negative, repays positive) to compute outstanding after
    /// all flows on each date have been processed.
    pub fn outstanding_by_date_including_notional(&self) -> Vec<(Date, Money)> {
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
                    CFKind::Notional => {
                        // Draws negative, repays positive -> subtract to apply sign
                        outstanding = outstanding.checked_sub(self.flows[j].amount).unwrap();
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

}
