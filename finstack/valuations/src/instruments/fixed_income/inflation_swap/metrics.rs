//! Placeholder metrics for Inflation Swap.
//!
//! These calculators are scaffolding and return trivial values so the
//! instrument can be wired into the metrics registry without full
//! implementation yet.

use crate::instruments::fixed_income::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::F;
use std::sync::Arc;

/// Breakeven inflation estimate (placeholder)
pub struct BreakevenCalculator;

impl MetricCalculator for BreakevenCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let _s: &InflationSwap = context.instrument_as()?;
        Ok(0.0)
    }
}

/// Fixed leg PV (placeholder)
pub struct FixedLegPvCalculator;

impl MetricCalculator for FixedLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let _s: &InflationSwap = context.instrument_as()?;
        Ok(0.0)
    }
}

/// Inflation leg PV (placeholder)
pub struct InflationLegPvCalculator;

impl MetricCalculator for InflationLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let _s: &InflationSwap = context.instrument_as()?;
        Ok(0.0)
    }
}

/// Register all inflation swap metrics with the registry
pub fn register_inflation_swap_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(
            MetricId::custom("breakeven"),
            Arc::new(BreakevenCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::custom("fixed_leg_pv"),
            Arc::new(FixedLegPvCalculator),
            &["InflationSwap"],
        )
        .register_metric(
            MetricId::custom("inflation_leg_pv"),
            Arc::new(InflationLegPvCalculator),
            &["InflationSwap"],
        );
}
