//! Gamma calculator for equity options.
//!
//! Uses greeks from the pricing engine to ensure consistency with PV.

use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;
        let greeks = crate::instruments::equity_option::pricing::engine::compute_greeks(
            option,
            &context.curves,
            context.as_of,
        )?;
        Ok(greeks.gamma)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}


