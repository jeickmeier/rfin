//! Equity metric: dividend yield (annualized, decimal).

use crate::instruments::equity::Equity;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Computes the dividend yield using `{ticker}-DIVYIELD` if present, or 0.0.
pub struct DividendYieldCalculator;

impl MetricCalculator for DividendYieldCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let equity: &Equity = context.instrument_as()?;
        let pricer = crate::instruments::equity::pricing::EquityPricer;
        pricer.dividend_yield(equity, &context.curves)
    }
}
