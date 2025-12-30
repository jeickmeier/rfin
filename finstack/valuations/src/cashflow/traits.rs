//! Cashflow-related traits and aliases.

use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::cashflow::builder::Notional;
use crate::cashflow::primitives::{CFKind, CashFlow};
pub use crate::cashflow::{DatedFlow, DatedFlows};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Build cashflow schedules and provide currency-safe aggregation hooks.
///
/// Instruments implement this to generate their cashflow schedules
/// given market curves and valuation date.
pub trait CashflowProvider: Send + Sync {
    /// Returns the instrument's notional amount, if applicable.
    ///
    /// Instruments with a defined notional should override this to return
    /// their principal amount. For multi-leg instruments (e.g., swaps),
    /// this typically returns the primary/receive leg notional.
    ///
    /// Default returns `None`, indicating the instrument doesn't have
    /// a simple notional concept or hasn't implemented this method.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::Money;
    /// use finstack_valuations::cashflow::traits::{CashflowProvider, DatedFlows};
    ///
    /// struct MyInstrument {
    ///     notional: Money,
    /// }
    ///
    /// impl CashflowProvider for MyInstrument {
    ///     fn build_full_schedule(
    ///         &self,
    ///         _curves: &MarketContext,
    ///         _as_of: Date,
    ///     ) -> finstack_core::Result<CashFlowSchedule> {
    ///         Ok(schedule_from_dated_flows(vec![], Some(self.notional)))
    ///     }
    ///
    ///     fn notional(&self) -> Option<Money> {
    ///         Some(self.notional)
    ///     }
    /// }
    ///
    /// let inst = MyInstrument { notional: Money::new(1_000_000.0, Currency::USD) };
    /// assert_eq!(inst.notional().unwrap().currency(), Currency::USD);
    /// ```
    fn notional(&self) -> Option<Money> {
        None
    }

    /// Build full cashflow schedule with CFKind metadata and outstanding tracking.
    ///
    /// This enhanced method provides complete cashflow information including:
    /// - Precise cashflow classification via `CFKind` (Interest, Principal, Fees, etc.)
    /// - Outstanding balance tracking over time
    /// - Notional amortization schedules
    ///
    /// Implementers should return their canonical [`CashFlowSchedule`]. Callers that only need
    /// `(Date, Money)` pairs can rely on [`CashflowProvider::build_dated_flows`] which converts
    /// this schedule automatically.
    ///
    /// # Errors
    /// Returns an error if the schedule cannot be built due to invalid
    /// instrument parameters or missing market data.
    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule>;

    /// Convenience: build holder-view `(Date, Money)` flows derived from the full schedule.
    ///
    /// Most callers that previously used [`CashflowProvider::build_schedule`] should call this
    /// helper, which simply converts the [`CashFlowSchedule`] returned by
    /// [`CashflowProvider::build_full_schedule`] into a `Vec<(Date, Money)>`.
    fn build_dated_flows(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        let schedule = self.build_full_schedule(curves, as_of)?;
        Ok(schedule
            .flows
            .iter()
            .map(|cf| (cf.date, cf.amount))
            .collect())
    }
}

/// Helper to convert holder-view `(Date, Money)` flows into a [`CashFlowSchedule`].
///
/// This mirrors the legacy default implementation and can be used by instruments that
/// naturally produce dated flows but still need to return a `CashFlowSchedule`.
pub fn schedule_from_dated_flows(
    flows: DatedFlows,
    notional_hint: Option<Money>,
) -> CashFlowSchedule {
    if flows.is_empty() {
        let ccy = notional_hint.map(|n| n.currency()).unwrap_or(Currency::USD);
        return CashFlowSchedule {
            flows: vec![],
            notional: Notional::par(0.0, ccy),
            day_count: DayCount::Act365F,
            meta: Default::default(),
        };
    }

    let first_currency = flows
        .first()
        .map(|(_, m)| m.currency())
        .unwrap_or(Currency::USD);
    let (notional_amount, notional_currency) = match notional_hint {
        Some(m) => (m.amount(), m.currency()),
        None => (0.0, first_currency),
    };

    let cf_flows: Vec<CashFlow> = flows
        .into_iter()
        .map(|(date, amount)| CashFlow {
            date,
            reset_date: None,
            amount,
            kind: CFKind::Fixed,
            accrual_factor: 0.0,
            rate: None,
        })
        .collect();

    CashFlowSchedule {
        flows: cf_flows,
        notional: Notional::par(notional_amount, notional_currency),
        day_count: DayCount::Act365F,
        meta: Default::default(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::money::Money;
    use time::Month;

    struct DummyInstrument;

    impl CashflowProvider for DummyInstrument {
        fn notional(&self) -> Option<Money> {
            Some(Money::new(1_000_000.0, Currency::USD))
        }

        fn build_full_schedule(
            &self,
            _curves: &MarketContext,
            _as_of: Date,
        ) -> finstack_core::Result<CashFlowSchedule> {
            let d1 = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
            let d2 = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");
            let flows = vec![
                (d1, Money::new(100.0, Currency::USD)),
                (d2, Money::new(250.0, Currency::USD)),
            ];
            Ok(schedule_from_dated_flows(flows, self.notional()))
        }
    }

    #[test]
    fn build_dated_flows_matches_schedule_contents() {
        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let dummy = DummyInstrument;
        let holder_flows = dummy
            .build_dated_flows(&curves, as_of)
            .expect("should build flows");
        assert_eq!(holder_flows.len(), 2);
        assert_eq!(holder_flows[0].1.amount(), 100.0);
        assert_eq!(holder_flows[1].1.amount(), 250.0);
    }

    #[test]
    fn schedule_from_dated_flows_uses_notional_hint() {
        let flows = vec![(
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            Money::new(100.0, Currency::USD),
        )];
        let notional = Money::new(5_000_000.0, Currency::USD);
        let schedule = schedule_from_dated_flows(flows, Some(notional));
        assert_eq!(schedule.notional.initial.amount(), 5_000_000.0);
        assert_eq!(schedule.notional.initial.currency(), Currency::USD);
    }

    #[test]
    fn schedule_from_dated_flows_defaults_currency() {
        let flows = vec![(
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            Money::new(100.0, Currency::EUR),
        )];
        let schedule = schedule_from_dated_flows(flows, None);
        assert_eq!(schedule.notional.initial.amount(), 0.0);
        assert_eq!(schedule.notional.initial.currency(), Currency::EUR);
    }
}
