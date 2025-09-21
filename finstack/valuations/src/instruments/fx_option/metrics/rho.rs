//! Rho calculators for FX options.

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

pub struct RhoDomesticCalculator;

impl MetricCalculator for RhoDomesticCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let greeks = crate::instruments::fx_option::pricing::compute_greeks(
            option,
            &context.curves,
            context.as_of,
        )?;
        Ok(greeks.rho_domestic)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}

pub struct RhoForeignCalculator;

impl MetricCalculator for RhoForeignCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &FxOption = context.instrument_as()?;
        let greeks = crate::instruments::fx_option::pricing::compute_greeks(
            option,
            &context.curves,
            context.as_of,
        )?;
        Ok(greeks.rho_foreign)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}


