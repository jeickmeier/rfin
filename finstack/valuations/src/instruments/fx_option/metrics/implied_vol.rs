//! Implied volatility metric for FX options.
//!
//! Solves for σ such that model PV(σ) equals the instrument's base PV
//! already computed in the `MetricContext`. Uses the configured pricer
//! (Hybrid solver under the hood) with log-σ parameterization.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxOption = context.instrument_as()?;
        let target = context.base_value.amount();
        // Use the instrument's implied vol method with current PV as target
        option.implied_vol(&context.curves, context.as_of, target, None)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
