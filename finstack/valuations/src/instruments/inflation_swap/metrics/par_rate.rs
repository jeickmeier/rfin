//! Par rate metric for `InflationSwap`.

use crate::instruments::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Calculates the par real rate for an inflation swap.
pub struct ParRateCalculator;

impl MetricCalculator for ParRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let swap: &InflationSwap = context.instrument_as()?;
        swap.par_rate(context.curves.as_ref())
    }
}
