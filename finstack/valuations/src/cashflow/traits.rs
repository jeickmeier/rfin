//! Cashflow-related traits and aliases.

use crate::instruments::fixed_income::discountable::Discountable;
use finstack_core::market_data::traits::Discount;
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
    /// # Errors
    /// Returns an error if the schedule cannot be built due to invalid
    /// instrument parameters or missing market data.
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows>;

    /// Convenience: present value the built schedule against a discount curve and day-count.
    ///
    /// See unit tests and `examples/` for usage.
    fn npv_with(
        &self,
        curves: &MarketContext,
        as_of: Date,
        disc: &dyn Discount,
        dc: DayCount,
    ) -> finstack_core::Result<Money> {
        let base = disc.base_date();
        let flows = self.build_schedule(curves, as_of)?;
        flows.npv(disc, base, dc)
    }
}
