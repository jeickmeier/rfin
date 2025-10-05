//! Equity metrics module.
//!
//! Provides metric calculators specific to `Equity`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_equity_metrics`.
//!
//! Exposed metrics:
//! - price per share
//! - shares
//! - dividend yield  
//! - forward price

mod dividend_yield;
mod forward_price;
mod price_per_share;
mod shares;

use crate::metrics::MetricRegistry;

/// Register all Equity metrics with the registry
pub fn register_equity_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "Equity",
        metrics: [
            (EquityPricePerShare, price_per_share::PricePerShareCalculator),
            (EquityShares, shares::SharesCalculator),
            (EquityDividendYield, dividend_yield::DividendYieldCalculator),
            (EquityForwardPrice, forward_price::ForwardPricePerShareCalculator),
        ]
    }
}
