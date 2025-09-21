//! Equity metric: price per share.

use crate::instruments::equity::Equity;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Computes the price per share for an `Equity`.
pub struct PricePerShareCalculator;

impl MetricCalculator for PricePerShareCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let equity: &Equity = context.instrument_as()?;
        let pricer = crate::instruments::equity::pricing::EquityPricer;
        let m = pricer.price_per_share(equity, &context.curves, context.as_of)?;
        Ok(m.amount())
    }
}
