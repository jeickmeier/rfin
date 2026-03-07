//! Implied volatility calculator for equity options.
//!
//! Solves for σ such that model price(σ) equals a provided market price. The
//! market price can be supplied via instrument attributes:
//! - `market_price`: numeric value as string
//! - `market_price_id`: id of a scalar in `MarketContext`

use crate::instruments::equity::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;

        // Market price
        let market_price: f64 = if let Some(p) = option.attributes.get_meta("market_price") {
            p.parse().unwrap_or(0.0)
        } else if let Some(price_id) = option.attributes.get_meta("market_price_id") {
            match context.curves.get_price(price_id) {
                Ok(ms) => match ms {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        option.implied_vol(&context.curves, context.as_of, market_price)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
