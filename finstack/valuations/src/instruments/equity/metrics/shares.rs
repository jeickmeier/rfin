//! Equity metric: shares (effective).

use crate::instruments::equity::Equity;
use crate::metrics::{MetricCalculator, MetricContext};


/// Computes the effective number of shares for an `Equity`.
pub struct SharesCalculator;

impl MetricCalculator for SharesCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let equity: &Equity = context.instrument_as()?;
        Ok(equity.effective_shares())
    }
}
