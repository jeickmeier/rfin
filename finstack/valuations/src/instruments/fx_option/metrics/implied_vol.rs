//! Implied volatility metric for FX options.
//!
//! Solves for σ such that model PV(σ) equals the instrument's base PV
//! already computed in the `MetricContext`. Uses the configured pricer
//! (Hybrid solver under the hood) with log-σ parameterization.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let target = context.base_value.amount();
        // Delegate to pricer's implied vol with default config, target is current PV
        let pricer = crate::instruments::fx_option::pricing::FxOptionPricer::default();
        pricer.implied_vol(option, &context.curves, context.as_of, target, None)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}


