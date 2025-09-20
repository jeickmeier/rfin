//! Premium/discount metric calculator.
//!
//! Computes percentage premium/discount of market price to NAV when a ticker
//! is configured and a price is available in `MarketContext`.

use crate::instruments::basket::types::Basket;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate premium/discount to NAV (requires market price)
pub struct PremiumDiscountCalculator;

impl MetricCalculator for PremiumDiscountCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        if let Some(ticker) = &basket.ticker {
            if let Ok(market_scalar) = context.curves.price(ticker) {
                let market_price = match market_scalar {
                    finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                };
                let nav = basket.nav(&context.curves, context.as_of)?;
                let premium_discount = (market_price / nav.amount() - 1.0) * 100.0;
                return Ok(premium_discount);
            }
        }
        Ok(0.0)
    }

    fn dependencies(&self) -> &[crate::metrics::MetricId] { &[crate::metrics::MetricId::Nav] }
}


