//! Variance swap pricing entrypoints and engine.
//!
//! This module implements the pricing logic for `VarianceSwap` and keeps it
//! separate from the instrument data structures in `types.rs`. It follows the
//! standard valuations layout used across instruments: `pricing/` contains the
//! pricing facade and engine implementation, and `metrics/` contains metric
//! calculators.

mod engine;

use crate::instruments::helpers::build_with_metrics_dyn;
use crate::instruments::traits::Priceable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use super::types::VarianceSwap;

impl Priceable for VarianceSwap {
    fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        engine::price(self, context, as_of)
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


