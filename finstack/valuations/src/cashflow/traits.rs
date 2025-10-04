//! Cashflow-related traits and aliases.

use crate::instruments::common::discountable::Discountable;
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;

/// Currency-preserving schedule as a list of dated `Money` amounts.
///
/// Used for cashflow aggregation and NPV calculations across different
/// instruments and time periods.
pub type DatedFlows = Vec<(Date, Money)>;

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
        use crate::cashflow::primitives::{CFKind, CashFlow, Notional};
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
            })
            .collect();

        // Estimate notional from largest flow (rough approximation)
        let max_amount = cf_flows
            .iter()
            .map(|cf| cf.amount.amount().abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

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
