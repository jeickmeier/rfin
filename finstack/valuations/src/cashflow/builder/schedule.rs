//! Schedule generation from the builder state.
//!
//! Provides the canonical `CashFlowSchedule` type and helpers for sorting and
//! deriving schedule metadata. Downstream pricing/risk code consumes this shape.

use crate::cashflow::primitives::Notional;
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;

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
}
