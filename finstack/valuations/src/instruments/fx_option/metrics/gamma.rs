//! Gamma calculator for FX options.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let greeks = option.compute_greeks(&context.curves, context.as_of)?;
        Ok(greeks.gamma)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
