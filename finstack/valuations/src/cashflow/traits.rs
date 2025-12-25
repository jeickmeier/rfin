//! Cashflow-related traits and aliases.

pub use crate::cashflow::{DatedFlow, DatedFlows};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Build cashflow schedules and provide currency-safe aggregation hooks.
///
/// Instruments implement this to generate their cashflow schedules
/// given market curves and valuation date.
pub trait CashflowProvider: Send + Sync {
    /// Build complete dated cashflow schedule as `(date, amount)` pairs.
    ///
    /// This is the simplified interface that provides basic cashflow information
    /// without metadata (CFKind, notional tracking, etc.).
    ///
    /// # Errors
    /// Returns an error if the schedule cannot be built due to invalid
    /// instrument parameters or missing market data.
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows>;

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
    ///     fn build_schedule(&self, _curves: &MarketContext, _as_of: Date) -> finstack_core::Result<DatedFlows> {
    ///         Ok(vec![])
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
    /// Default implementation converts `build_schedule()` output to a basic schedule
    /// without CFKind information (heuristic-based classification required).
    /// Instruments should override this to provide precise classification.
    ///
    /// # Errors
    /// Returns an error if the schedule cannot be built due to invalid
    /// instrument parameters or missing market data.
    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        use crate::cashflow::builder::schedule::CashFlowSchedule;
        use crate::cashflow::builder::Notional;
        use finstack_core::cashflow::primitives::{CFKind, CashFlow};
        use finstack_core::dates::DayCount;

        // Default implementation: convert simple flows to basic schedule
        // Individual instruments should override this for precise classification
        let flows = self.build_schedule(curves, as_of)?;

        if flows.is_empty() {
            // Use currency from notional if available, otherwise fallback to USD
            let ccy = self
                .notional()
                .map(|n| n.currency())
                .unwrap_or(finstack_core::currency::Currency::USD);
            return Ok(CashFlowSchedule {
                flows: vec![],
                notional: Notional::par(0.0, ccy),
                day_count: DayCount::Act365F,
                meta: Default::default(),
            });
        }

        // Get currency from first flow for fallback
        let currency = flows[0].1.currency();

        // Convert (Date, Money) to CashFlow with generic CFKind
        // This loses classification precision but maintains compatibility
        let cf_flows: Vec<CashFlow> = flows
            .into_iter()
            .map(|(date, amount)| CashFlow {
                date,
                reset_date: None,
                amount,
                kind: CFKind::Fixed, // Generic - precise instruments should override
                accrual_factor: 0.0,
                rate: None,
            })
            .collect();

        // Use explicit notional from instrument if available, otherwise fallback to zero
        // (signals unknown notional rather than guessing from cashflows)
        let notional = match self.notional() {
            Some(n) => Notional::par(n.amount(), n.currency()),
            None => Notional::par(0.0, currency),
        };

        Ok(CashFlowSchedule {
            flows: cf_flows,
            notional,
            day_count: DayCount::Act365F,
            meta: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::money::Money;
    use time::Month;

    /// Dummy with explicit notional
    struct DummyWithNotional;

    impl CashflowProvider for DummyWithNotional {
        fn build_schedule(
            &self,
            _curves: &MarketContext,
            _as_of: Date,
        ) -> finstack_core::Result<DatedFlows> {
            let d1 = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
            let d2 = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");
            Ok(vec![
                (d1, Money::new(100.0, Currency::USD)),
                (d2, Money::new(250.0, Currency::USD)),
            ])
        }

        fn notional(&self) -> Option<Money> {
            Some(Money::new(1_000_000.0, Currency::USD))
        }
    }

    /// Dummy without notional implementation (uses default)
    struct DummyWithoutNotional;

    impl CashflowProvider for DummyWithoutNotional {
        fn build_schedule(
            &self,
            _curves: &MarketContext,
            _as_of: Date,
        ) -> finstack_core::Result<DatedFlows> {
            let d1 = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
            let d2 = Date::from_calendar_date(2025, Month::July, 1).expect("valid date");
            Ok(vec![
                (d1, Money::new(100.0, Currency::USD)),
                (d2, Money::new(250.0, Currency::USD)),
            ])
        }
    }

    #[test]
    fn full_schedule_uses_explicit_notional() {
        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let dummy = DummyWithNotional;
        let sched = dummy
            .build_full_schedule(&curves, as_of)
            .expect("should build schedule");
        // Uses the explicit notional from the trait method
        assert_eq!(sched.notional.initial.amount(), 1_000_000.0);
        assert_eq!(sched.notional.initial.currency(), Currency::USD);
        assert_eq!(sched.day_count, finstack_core::dates::DayCount::Act365F);
        assert_eq!(sched.flows.len(), 2);
    }

    #[test]
    fn full_schedule_fallback_to_zero_notional() {
        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let dummy = DummyWithoutNotional;
        let sched = dummy
            .build_full_schedule(&curves, as_of)
            .expect("should build schedule");
        // Falls back to 0.0 notional when not implemented
        assert_eq!(sched.notional.initial.amount(), 0.0);
        assert_eq!(sched.notional.initial.currency(), Currency::USD);
        assert_eq!(sched.day_count, finstack_core::dates::DayCount::Act365F);
        assert_eq!(sched.flows.len(), 2);
    }
}
