//! Rho calculator for equity options (cash sensitivity per 1% rate).

use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;
        let greeks = crate::instruments::equity_option::pricing::engine::compute_greeks(
            option,
            &context.curves,
            context.as_of,
        )?;
        Ok(greeks.rho)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}


