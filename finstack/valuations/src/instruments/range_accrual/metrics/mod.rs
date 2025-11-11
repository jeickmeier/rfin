//! Range accrual metrics module.
//!
//! Provides full greek coverage for range accrual instruments using
//! finite difference methods. Delta and Gamma use generic FD calculators.
//! Includes bucketed DV01 for detailed interest rate risk analysis.

// mod dv01; // removed - using GenericParallelDv01
#[cfg(feature = "mc")]
mod rho;
#[cfg(feature = "mc")]
mod vanna;
#[cfg(feature = "mc")]
mod vega;
#[cfg(feature = "mc")]
mod volga;

#[cfg(feature = "mc")]
use crate::metrics::{MetricId, MetricRegistry};
#[cfg(feature = "mc")]
use std::sync::Arc;

/// Register range accrual metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_range_accrual_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{GenericFdDelta, GenericFdGamma};

    // Use generic FD calculators for Delta and Gamma
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<crate::instruments::RangeAccrual>::default()),
        &["RangeAccrual"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::RangeAccrual>::default()),
        &["RangeAccrual"],
    );

    // Other metrics use custom implementations
    #[cfg(feature = "mc")]
    {
        crate::register_metrics! {
            registry: registry,
            instrument: "RangeAccrual",
            metrics: [
                (Vega, vega::VegaCalculator::default()),
                (Rho, rho::RhoCalculator),
                (Dv01, crate::metrics::GenericParallelDv01::<
                    crate::instruments::range_accrual::RangeAccrual,
                >::default()),
                (Vanna, vanna::VannaCalculator),
                (Volga, volga::VolgaCalculator::default()),
                // Theta is now registered universally in metrics::standard_registry()
                (BucketedDv01, crate::metrics::GenericBucketedDv01WithContext::<
                    crate::instruments::RangeAccrual,
                >::default()),
            ]
        }
    }
}
