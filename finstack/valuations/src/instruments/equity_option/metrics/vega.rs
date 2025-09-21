//! Vega calculator for equity options (cash vega per 1% vol).

use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;
        let greeks = crate::instruments::equity_option::pricing::engine::compute_greeks(
            option,
            &context.curves,
            context.as_of,
        )?;
        Ok(greeks.vega)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
