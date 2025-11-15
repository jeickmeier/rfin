//! Cashflow-related traits and aliases.

pub use crate::cashflow::{DatedFlow, DatedFlows};
use crate::instruments::common::discountable::Discountable;
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;

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
            return Ok(CashFlowSchedule {
                flows: vec![],
                notional: Notional::par(0.0, finstack_core::currency::Currency::USD),
                day_count: DayCount::Act365F,
                meta: Default::default(),
            });
        }

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

        // Estimate notional from largest flow (rough approximation)
        let max_amount = cf_flows
            .iter()
            .map(|cf| cf.amount.amount().abs())
            .fold(0.0, f64::max);

        let currency = cf_flows[0].amount.currency();
        let notional = Notional::par(max_amount, currency);

        Ok(CashFlowSchedule {
            flows: cf_flows,
            notional,
            day_count: DayCount::Act365F,
            meta: Default::default(),
        })
    }

    /// Convenience: present value the built schedule against a discount curve and day-count.
    ///
    /// See unit tests and `examples/` for usage.
    fn npv_with(
        &self,
        curves: &MarketContext,
        as_of: Date,
        disc: &dyn Discounting,
        dc: DayCount,
    ) -> finstack_core::Result<Money> {
        let base = disc.base_date();
        let flows = self.build_schedule(curves, as_of)?;
        flows.npv(disc, base, dc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::MarketContext;
    use finstack_core::money::Money;
    use time::Month;

    struct Dummy;

    impl CashflowProvider for Dummy {
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
    fn full_schedule_default_max_amount_ok() {
        let curves = MarketContext::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let dummy = Dummy;
        let sched = dummy.build_full_schedule(&curves, as_of).expect("should build schedule");
        assert_eq!(sched.notional.initial.amount(), 250.0);
        assert_eq!(sched.notional.initial.currency(), Currency::USD);
        assert_eq!(sched.day_count, finstack_core::dates::DayCount::Act365F);
        assert_eq!(sched.flows.len(), 2);
    }
}
