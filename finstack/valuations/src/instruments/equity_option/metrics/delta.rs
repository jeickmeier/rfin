//! Delta calculator for equity options.
//!
//! Computes cash delta using Black–Scholes greeks from the pricing engine.

use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;
        let greeks = crate::instruments::equity_option::pricing::engine::compute_greeks(
            option,
            &context.curves,
            context.as_of,
        )?;
        Ok(greeks.delta)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
