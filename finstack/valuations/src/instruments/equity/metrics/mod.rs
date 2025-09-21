//! Equity metrics module.
//!
//! Provides metric calculators specific to `Equity`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_equity_metrics`.
//!
//! Exposed metrics:
//! - price per share
//! - shares
//! - market value

mod dividend_yield;
mod forward_price;
mod market_value;
mod price_per_share;
mod shares;

use crate::metrics::MetricRegistry;

/// Register all Equity metrics with the registry
pub fn register_equity_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry.register_metric(
        MetricId::EquityPricePerShare,
        Arc::new(price_per_share::PricePerShareCalculator),
        &["Equity"],
    );
    registry.register_metric(
        MetricId::EquityShares,
        Arc::new(shares::SharesCalculator),
        &["Equity"],
    );
    registry.register_metric(
        MetricId::EquityMarketValue,
        Arc::new(market_value::MarketValueCalculator),
        &["Equity"],
    );
    registry.register_metric(
        MetricId::EquityDividendYield,
        Arc::new(dividend_yield::DividendYieldCalculator),
        &["Equity"],
    );
    registry.register_metric(
        MetricId::EquityForwardPrice,
        Arc::new(forward_price::ForwardPricePerShareCalculator),
        &["Equity"],
    );
}
