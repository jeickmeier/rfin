//! Autocallable metrics module.
//!
//! Provides full greek coverage for autocallable structured products using
//! finite difference methods. Delta and Gamma use generic FD calculators.
//! Note: Autocallables exhibit discontinuities near autocall barrier levels.

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

/// Register autocallable metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_autocallable_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{GenericFdDelta, GenericFdGamma, GenericFdVanna, GenericFdVolga};
    use crate::pricer::InstrumentType;

    // Use generic FD calculators for Delta, Gamma, Vanna, and Volga
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<crate::instruments::Autocallable>::default()),
        &[InstrumentType::Autocallable],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::Autocallable>::default()),
        &[InstrumentType::Autocallable],
    );

    registry.register_metric(
        MetricId::Vanna,
        Arc::new(GenericFdVanna::<crate::instruments::Autocallable>::default()),
        &[InstrumentType::Autocallable],
    );

    registry.register_metric(
        MetricId::Volga,
        Arc::new(GenericFdVolga::<crate::instruments::Autocallable>::default()),
        &[InstrumentType::Autocallable],
    );

    // Other metrics use custom implementations
    #[cfg(feature = "mc")]
    {
        crate::register_metrics! {
            registry: registry,
            instrument: InstrumentType::Autocallable,
            metrics: [
                (Vega, vega::VegaCalculator::default()),
                (Rho, rho::RhoCalculator::default()),
                (Dv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::Autocallable,
                >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
                (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::Autocallable,
                >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
                // Theta is now registered universally in metrics::standard_registry()
            ]
        }
    }
}
