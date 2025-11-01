//! Rho calculators for FX options (per 1bp).

use crate::instruments::fx_option::FxOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

pub struct RhoDomesticCalculator;

impl MetricCalculator for RhoDomesticCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxOption = context.instrument_as()?;
        let greeks = option.compute_greeks(&context.curves, context.as_of)?;
        // Greeks are per 1%; convert to per 1bp
        Ok(greeks.rho_domestic / 100.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

pub struct RhoForeignCalculator;

impl MetricCalculator for RhoForeignCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxOption = context.instrument_as()?;
        let greeks = option.compute_greeks(&context.curves, context.as_of)?;
        // Greeks are per 1%; convert to per 1bp
        Ok(greeks.rho_foreign / 100.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
