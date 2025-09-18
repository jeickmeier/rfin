//! Bond pricing entrypoints and pricers.

mod discount;
pub mod helpers;
pub mod oas_pricer;
pub mod ytm_solver;

use crate::instruments::traits::Priceable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::instruments::utils::build_with_metrics_dyn;
use super::types::Bond;

impl Priceable for Bond {
    fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        discount::price(self, context, as_of)
    }

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


