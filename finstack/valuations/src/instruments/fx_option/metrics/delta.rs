//! Delta calculator for FX options.
//!
//! Computes cash delta using Garman–Kohlhagen greeks from the pricing engine.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result};

pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxOption = context.instrument_as()?;
        let greeks = option.compute_greeks(&context.curves, context.as_of)?;
        Ok(greeks.delta)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
