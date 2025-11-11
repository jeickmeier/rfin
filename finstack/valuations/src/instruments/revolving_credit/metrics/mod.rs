//! Metrics module for revolving credit facilities.
//!
//! Provides both standard metrics (PV, DV01, Theta, BucketedDV01, CS01) and
//! facility-specific metrics (utilization rate, available capacity, weighted average cost, IRR).

pub mod available_capacity;
pub mod irr;
pub mod utilization_rate;
pub mod weighted_average_cost;

pub use available_capacity::AvailableCapacityCalculator;
pub use irr::{calculate_path_irr, calculate_periodic_irr};
pub use utilization_rate::UtilizationRateCalculator;
pub use weighted_average_cost::ApproxWeightedAverageCostCalculator;

/// Backward compatibility alias for weighted average cost calculator.
///
/// Prefer using `ApproxWeightedAverageCostCalculator` directly for new code.
pub type WeightedAverageCostCalculator = ApproxWeightedAverageCostCalculator;

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
            (Dv01, crate::metrics::GenericParallelDv01::<
                crate::instruments::RevolvingCredit,
            >::default()),
            (Cs01, crate::metrics::GenericParallelCs01::<
                crate::instruments::RevolvingCredit,
            >::default()),
            // Theta is now registered universally in metrics::standard_registry()
            (BucketedDv01, crate::metrics::GenericBucketedDv01WithContext::<
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
        Arc::new(ApproxWeightedAverageCostCalculator),
        &["RevolvingCredit"],
    );
}
