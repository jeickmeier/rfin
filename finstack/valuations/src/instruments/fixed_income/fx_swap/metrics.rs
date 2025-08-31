//! FX Swap metrics (boilerplate registrations).
//!
//! Placeholder metric calculators for FX Swap. These provide minimal
//! scaffolding so the instrument can be priced with the metrics framework.

use crate::instruments::fixed_income::fx_swap::FxSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::F;

/// Near rate metric (if provided on instrument), else 0.0
pub struct NearRate;

impl MetricCalculator for NearRate {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx: &FxSwap = context.instrument_as()?;
        Ok(fx.near_rate.unwrap_or(0.0))
    }
}

/// Far rate metric (if provided on instrument), else 0.0
pub struct FarRate;

impl MetricCalculator for FarRate {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx: &FxSwap = context.instrument_as()?;
        Ok(fx.far_rate.unwrap_or(0.0))
    }
}

/// Registers placeholder FX Swap metrics
pub fn register_fx_swap_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;

    registry
        .register_metric(MetricId::custom("near_rate"), Arc::new(NearRate), &["FxSwap"]) 
        .register_metric(MetricId::custom("far_rate"), Arc::new(FarRate), &["FxSwap"]);
}


