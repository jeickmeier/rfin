//! Metrics module for revolving credit facilities.
//!
//! Provides both standard metrics (PV, DV01, Theta, BucketedDV01, CS01) and
//! facility-specific metrics (utilization rate, available capacity, weighted average cost).

pub mod available_capacity;
pub mod cs01;
pub mod dv01;
pub mod utilization_rate;
pub mod weighted_average_cost;

pub use available_capacity::AvailableCapacityCalculator;
pub use cs01::Cs01Calculator;
pub use dv01::Dv01Calculator;
pub use utilization_rate::UtilizationRateCalculator;
pub use weighted_average_cost::WeightedAverageCostCalculator;

use crate::metrics::MetricRegistry;

/// Register all revolving credit metrics with the registry.
///
/// Registers both standard metrics (PV, DV01, Theta, BucketedDV01, CS01) and
/// facility-specific metrics (utilization rate, available capacity, weighted average cost).
pub fn register_revolving_credit_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "RevolvingCredit",
        metrics: [
            (Dv01, Dv01Calculator),
            (Cs01, Cs01Calculator),
            (Theta, crate::instruments::common::metrics::GenericTheta::<
                crate::instruments::RevolvingCredit,
            >::default()),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::RevolvingCredit,
            >::default()),
        ]
    }

    // Register facility-specific metrics with custom IDs
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry.register_metric(
        MetricId::custom("utilization_rate"),
        Arc::new(UtilizationRateCalculator),
        &["RevolvingCredit"],
    );

    registry.register_metric(
        MetricId::custom("available_capacity"),
        Arc::new(AvailableCapacityCalculator),
        &["RevolvingCredit"],
    );

    registry.register_metric(
        MetricId::custom("weighted_average_cost"),
        Arc::new(WeightedAverageCostCalculator),
        &["RevolvingCredit"],
    );
}
