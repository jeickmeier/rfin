//! Autocallable metrics module.
//!
//! Provides full greek coverage for autocallable structured products using
//! finite difference methods. Delta and Gamma use generic FD calculators.
//! Note: Autocallables exhibit discontinuities near autocall barrier levels.

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

/// Register autocallable metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_autocallable_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{GenericFdDelta, GenericFdGamma};

    // Use generic FD calculators for Delta and Gamma
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<crate::instruments::Autocallable>::default()),
        &["Autocallable"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::Autocallable>::default()),
        &["Autocallable"],
    );

    // Other metrics use custom implementations
    #[cfg(feature = "mc")]
    {
        crate::register_metrics! {
            registry: registry,
            instrument: "Autocallable",
            metrics: [
                (Vega, vega::VegaCalculator::default()),
                (Rho, rho::RhoCalculator),
                (Dv01, crate::metrics::GenericParallelDv01::<
                    crate::instruments::Autocallable,
                >::default()),
                (Vanna, vanna::VannaCalculator),
                (Volga, volga::VolgaCalculator::default()),
                // Theta is now registered universally in metrics::standard_registry()
            ]
        }
    }
}
