//! Deposit pricing entrypoints and pricer.
//!
//! Mirrors the structure used by other instruments (e.g., basis swap):
//! a small pricing facade that delegates to the core `DepositEngine`, and
//! a `Priceable` implementation that composes metrics via the shared helper.

pub mod engine;

use crate::instruments::deposit::types::Deposit;
use crate::instruments::helpers::build_with_metrics_dyn;
use crate::instruments::traits::Priceable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use engine::DepositEngine;

impl Priceable for Deposit {
    /// Calculates the present value of the deposit using the core engine.
    ///
    /// The PV is computed as the discounted value of two cashflows:
    /// - Outflow of principal at the start date
    /// - Inflow of principal plus simple interest at maturity
    fn value(&self, context: &MarketContext, _as_of: Date) -> Result<Money> {
        DepositEngine::pv(self, context)
    }

    /// Calculates the present value with additional metrics.
    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        let base = <Self as Priceable>::value(self, context, as_of)?;
        build_with_metrics_dyn(self, context, as_of, base, metrics)
    }
}
