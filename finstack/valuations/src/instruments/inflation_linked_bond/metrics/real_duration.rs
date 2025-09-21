//! ILB real duration metric calculator.

use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Real duration calculator for ILB
pub struct RealDurationCalculator;

impl MetricCalculator for RealDurationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let ilb: &InflationLinkedBond = context.instrument_as()?;
        ilb.real_duration(&context.curves, context.as_of)
    }
}
