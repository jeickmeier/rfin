//! Fixed leg PV metric for `InflationSwap`.

use crate::instruments::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Calculates PV of fixed leg for inflation swaps.
pub struct FixedLegPvCalculator;

impl MetricCalculator for FixedLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let s: &InflationSwap = context.instrument_as()?;
        let pv_fixed = s.pv_fixed_leg(&context.curves, context.as_of)?;
        Ok(pv_fixed.amount())
    }
}


