//! ILB metrics module.
//!
//! Splits ILB-specific metric calculators into focused files and
//! registers them with the shared metrics framework.

mod breakeven_inflation;
mod index_ratio;
mod real_duration;
mod real_yield;

pub use breakeven_inflation::BreakevenInflationCalculator;
pub use index_ratio::IndexRatioCalculator;
pub use real_duration::RealDurationCalculator;
pub use real_yield::RealYieldCalculator;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register all ILB metrics with the registry
pub fn register_ilb_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(
            MetricId::custom("real_yield"),
            Arc::new(RealYieldCalculator),
            &["ILB"],
        )
        .register_metric(
            MetricId::custom("index_ratio"),
            Arc::new(IndexRatioCalculator),
            &["ILB"],
        )
        .register_metric(
            MetricId::custom("real_duration"),
            Arc::new(RealDurationCalculator),
            &["ILB"],
        )
        .register_metric(
            MetricId::custom("breakeven_inflation"),
            Arc::new(BreakevenInflationCalculator),
            &["ILB"],
        );
}
