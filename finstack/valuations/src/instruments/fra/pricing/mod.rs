//! FRA pricing entrypoints and pricer.
//!
//! Mirrors the structure used by `deposit` and other instruments: a small
//! pricing facade that delegates to the core `FraEngine`, and a `Priceable`
//! implementation that composes metrics via the shared helper.

pub mod engine;

use crate::instruments::fra::types::ForwardRateAgreement;
use crate::instruments::helpers::build_with_metrics_dyn;
use crate::instruments::traits::Priceable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use engine::FraEngine;

impl Priceable for ForwardRateAgreement {
    /// Calculates the present value of the FRA using the core engine.
    ///
    /// Settlement is at the start of the accrual period; PV reflects the
    /// discounted payoff at that settlement date.
    fn value(&self, context: &MarketContext, _as_of: Date) -> Result<Money> {
        FraEngine::pv(self, context)
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
