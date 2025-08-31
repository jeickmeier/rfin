//! Equity metrics: price per share, shares, and market value.

use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::F;

struct PricePerShareCalculator;
impl MetricCalculator for PricePerShareCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let equity = context
            .instrument
            .as_equity()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        equity
            .price_quote
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))
    }
}

struct SharesCalculator;
impl MetricCalculator for SharesCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let equity = context
            .instrument
            .as_equity()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        Ok(equity.effective_shares())
    }
}

struct MarketValueCalculator;
impl MetricCalculator for MarketValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let equity = context
            .instrument
            .as_equity()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;
        let price = equity
            .price_quote
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::NotFound))?;
        Ok(price * equity.effective_shares())
    }
}

/// Register equity metrics in the standard registry
pub fn register_equity_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;
    registry.register_metric(
        MetricId::custom("price_per_share"),
        Arc::new(PricePerShareCalculator),
        &["Equity"],
    );
    registry.register_metric(
        MetricId::custom("shares"),
        Arc::new(SharesCalculator),
        &["Equity"],
    );
    registry.register_metric(
        MetricId::custom("market_value"),
        Arc::new(MarketValueCalculator),
        &["Equity"],
    );
}


