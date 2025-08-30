//! Schedule generation from the builder state.
//!
//! Provides the canonical `CashFlowSchedule` type and helpers for sorting and
//! deriving schedule metadata. Downstream pricing/risk code consumes this shape.

use crate::cashflow::amortization_notional::Notional;
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;

use super::types::{FixedCouponSpec, FloatingCouponSpec};

#[inline]
pub(crate) fn kind_rank(kind: CFKind) -> u8 {
    match kind {
        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
        CFKind::Fee => 1,
        CFKind::Amortization => 2,
        CFKind::PIK => 3,
        CFKind::Notional => 4,
    }
}

#[inline]
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

    let mut cals: Vec<&'static str> = Vec::new();
    for s in fixed {
        if let Some(id) = s.calendar_id {
            cals.push(id);
        }
    }
    for s in floating {
        if let Some(id) = s.calendar_id {
            cals.push(id);
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
    pub calendar_ids: Vec<&'static str>,
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
    /// Returns the list of dates for all flows in schedule order.
    #[inline]
    pub fn dates(&self) -> Vec<Date> {
        self.flows.iter().map(|cf| cf.date).collect()
    }

    /// Returns an iterator over flows of the given `CFKind`.
    #[inline]
    pub fn flows_of_kind(&self, kind: CFKind) -> impl Iterator<Item = &CashFlow> {
        self.flows.iter().filter(move |cf| cf.kind == kind)
    }

    /// Outstanding principal path computed from principal/PIK/amortization flows.
    /// Assumes economic signs: amortization negative, PIK positive, final notional positive redemption.
    ///
    /// Example
    /// -------
    /// ```rust
    /// use finstack_core::dates::Date;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::money::Money;
    /// use finstack_valuations::cashflow::builder::schedule::{CashFlowSchedule, CashflowMeta};
    /// use finstack_valuations::cashflow::primitives::{CashFlow, CFKind};
    /// use finstack_valuations::cashflow::amortization_notional::Notional;
    /// use time::Month;
    ///
    /// let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    /// let notional = Notional { initial: Money::new(100.0, Currency::USD), amort: Default::default() };
    /// let flows = vec![
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(-10.0, Currency::USD), kind: CFKind::Amortization, accrual_factor: 0.0 },
    ///   CashFlow { date: base, reset_date: None, amount: Money::new(  5.0, Currency::USD), kind: CFKind::PIK,          accrual_factor: 0.0 },
    /// ];
    /// let s = CashFlowSchedule { flows, notional, day_count: finstack_core::dates::DayCount::Act365F, meta: CashflowMeta::default() };
    /// let path = s.outstanding_path();
    /// assert_eq!(path.len(), 2);
    /// assert_eq!(path[0].1.amount(), 90.0);
    /// assert_eq!(path[1].1.amount(), 95.0);
    /// ```
    pub fn outstanding_path(&self) -> Vec<(Date, Money)> {
        let mut out = Vec::new();
        let mut outstanding = self.notional.initial.amount();
        let ccy = self.notional.initial.currency();
        for cf in &self.flows {
            match cf.kind {
                CFKind::Amortization => {
                    outstanding += cf.amount.amount(); // amount is negative
                }
                CFKind::PIK => {
                    outstanding += cf.amount.amount(); // adds to outstanding
                }
                _ => {}
            }
            out.push((cf.date, Money::new(outstanding, ccy)));
        }
        out
    }

    // Convenience iterators for callers to avoid ad-hoc filtering.
    #[inline]
    pub fn coupons(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub)
    }

    #[inline]
    pub fn amortizations(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Amortization)
    }

    #[inline]
    pub fn redemptions(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Notional && cf.amount.amount() > 0.0)
    }

    /// End-of-date outstanding path: one entry per unique date after applying
    /// Amortization/PIK on that date. Redemption does not reduce outstanding here.
    #[inline]
    pub fn outstanding_by_date(&self) -> Vec<(Date, Money)> {
        let mut result: Vec<(Date, Money)> = Vec::new();
        if self.flows.is_empty() {
            return result;
        }

        let ccy = self.notional.initial.currency();
        let mut outstanding = self.notional.initial.amount();

        let mut i = 0usize;
        while i < self.flows.len() {
            let d = self.flows[i].date;
            // Process all flows on this date in their deterministic order
            let mut j = i;
            while j < self.flows.len() && self.flows[j].date == d {
                match self.flows[j].kind {
                    CFKind::Amortization => {
                        outstanding += self.flows[j].amount.amount();
                    }
                    CFKind::PIK => {
                        outstanding += self.flows[j].amount.amount();
                    }
                    _ => {}
                }
                j += 1;
            }
            result.push((d, Money::new(outstanding, ccy)));
            i = j;
        }

        result
    }
}
