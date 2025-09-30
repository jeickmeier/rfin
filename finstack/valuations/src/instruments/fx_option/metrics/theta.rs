//! Theta calculator for FX options.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxOption = context.instrument_as()?;
        let greeks = option.compute_greeks(&context.curves, context.as_of)?;
        Ok(greeks.theta)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
