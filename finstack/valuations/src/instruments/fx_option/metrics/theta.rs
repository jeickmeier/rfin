//! Theta calculator for FX options.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let greeks = crate::instruments::fx_option::pricing::compute_greeks(
            option,
            &context.curves,
            context.as_of,
        )?;
        Ok(greeks.theta)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}


