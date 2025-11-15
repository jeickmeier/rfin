//! Barrier option metrics module.
//!
//! Provides full greek coverage for barrier options using finite difference methods.
//! Delta and Gamma use generic FD calculators.
//! Note: Barrier options exhibit discontinuous greeks near the barrier level.

// mod dv01; // removed - using GenericParallelDv01
// mod vanna; // removed - using GenericFdVanna
// mod volga; // removed - using GenericFdVolga
#[cfg(feature = "mc")]
mod rho;
#[cfg(feature = "mc")]
mod vega;

#[cfg(feature = "mc")]
use crate::metrics::{MetricId, MetricRegistry};
#[cfg(feature = "mc")]
use std::sync::Arc;

/// Register barrier option metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_barrier_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{GenericFdDelta, GenericFdGamma, GenericFdVanna, GenericFdVolga};

    // Use generic FD calculators for Delta, Gamma, Vanna, and Volga
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<crate::instruments::BarrierOption>::default()),
        &["BarrierOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::BarrierOption>::default()),
        &["BarrierOption"],
    );

    registry.register_metric(
        MetricId::Vanna,
        Arc::new(GenericFdVanna::<crate::instruments::BarrierOption>::default()),
        &["BarrierOption"],
    );

    registry.register_metric(
        MetricId::Volga,
        Arc::new(GenericFdVolga::<crate::instruments::BarrierOption>::default()),
        &["BarrierOption"],
    );

    // Other metrics use custom implementations
    #[cfg(feature = "mc")]
    {
        crate::register_metrics! {
            registry: registry,
            instrument: "BarrierOption",
            metrics: [
                (Vega, vega::VegaCalculator::default()),
                (Rho, rho::RhoCalculator),
                (Dv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::BarrierOption,
                >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
                (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::BarrierOption,
                >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
                // Theta is now registered universally in metrics::standard_registry()
            ]
        }
    }
}
