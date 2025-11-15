//! Barrier option metrics module.
//!
//! Provides full greek coverage for barrier options using finite difference methods.
//! Delta and Gamma use generic FD calculators.
//! Note: Barrier options exhibit discontinuous greeks near the barrier level.

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

/// Register barrier option metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_barrier_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{GenericFdDelta, GenericFdGamma};

    // Use generic FD calculators for Delta and Gamma
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
                (Vanna, vanna::VannaCalculator),
                (Volga, volga::VolgaCalculator::default()),
                // Theta is now registered universally in metrics::standard_registry()
            ]
        }
    }
}
