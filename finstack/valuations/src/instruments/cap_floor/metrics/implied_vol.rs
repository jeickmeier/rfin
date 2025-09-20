//! Implied volatility calculator for interest rate options.
//!
//! Placeholder implementation: for full functionality we would root-find the
//! Black volatility that reproduces the observed option PV.

use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

/// Implied volatility calculator (placeholder)
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &InterestRateOption = context.instrument_as()?;
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
