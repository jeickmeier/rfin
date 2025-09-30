//! Par rate metric for `InflationSwap`.

use crate::instruments::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};

/// Calculates the par real rate for an inflation swap.
pub struct ParRateCalculator;

impl MetricCalculator for ParRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let swap: &InflationSwap = context.instrument_as()?;
        swap.par_rate(context.curves.as_ref())
    }
}
