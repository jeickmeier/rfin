//! ILB metrics module.
//!
//! Splits ILB-specific metric calculators into focused files and
//! registers them with the shared metrics framework.

mod breakeven_inflation;
mod dv01;
mod index_ratio;
mod inflation01;
mod inflation_convexity;
mod real_duration;
mod real_yield;
// mod theta; // removed - using GenericThetaAny
// risk_bucketed_dv01 - now using generic implementation

pub use breakeven_inflation::BreakevenInflationCalculator;
pub use index_ratio::IndexRatioCalculator;
pub use inflation01::Inflation01Calculator;
pub use inflation_convexity::InflationConvexityCalculator;
pub use real_duration::RealDurationCalculator;
pub use real_yield::RealYieldCalculator;
// pub use theta::ThetaCalculator; // removed - using GenericThetaAny
// BucketedDv01Calculator now using generic implementation

use crate::metrics::MetricId;
use crate::metrics::MetricRegistry;
use std::sync::Arc;

/// Register all ILB metrics with the registry
pub fn register_ilb_metrics(registry: &mut MetricRegistry) {
    // Custom metric: Inflation01 (inflation curve sensitivity per 1bp)
    registry.register_metric(
        MetricId::Inflation01,
        Arc::new(Inflation01Calculator),
        &["InflationLinkedBond"],
    );

    // Custom metric: InflationConvexity (second-order inflation sensitivity)
    registry.register_metric(
        MetricId::InflationConvexity,
        Arc::new(InflationConvexityCalculator),
        &["InflationLinkedBond"],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: "InflationLinkedBond",
        metrics: [
            (RealYield, RealYieldCalculator),
            (IndexRatio, IndexRatioCalculator),
            (RealDuration, RealDurationCalculator),
            (BreakevenInflation, BreakevenInflationCalculator),
            (Dv01, dv01::InflationLinkedBondDv01Calculator),
            // Theta is now registered universally in metrics::standard_registry()
            (BucketedDv01, crate::metrics::GenericBucketedDv01::<
                crate::instruments::InflationLinkedBond,
            >::default()),
        ]
    };
}
