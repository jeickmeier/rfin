//! Theta calculator for equity options (cash theta per day by convention).

use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;
        let greeks = crate::instruments::equity_option::pricing::engine::compute_greeks(
            option,
            &context.curves,
            context.as_of,
        )?;
        Ok(greeks.theta)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
