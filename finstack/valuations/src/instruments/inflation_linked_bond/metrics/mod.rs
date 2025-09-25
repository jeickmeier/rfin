//! ILB metrics module.
//!
//! Splits ILB-specific metric calculators into focused files and
//! registers them with the shared metrics framework.

mod breakeven_inflation;
mod index_ratio;
mod real_duration;
mod real_yield;
mod risk_bucketed_dv01;

pub use breakeven_inflation::BreakevenInflationCalculator;
pub use index_ratio::IndexRatioCalculator;
pub use real_duration::RealDurationCalculator;
pub use real_yield::RealYieldCalculator;
pub use risk_bucketed_dv01::BucketedDv01Calculator;

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
        )
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(BucketedDv01Calculator),
            &["ILB"],
        );
}
