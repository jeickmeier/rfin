//! Lookback option metrics module.
//!
//! Provides full greek coverage for lookback options using finite difference methods.
//! Delta and Gamma use generic FD calculators.

// mod dv01; // removed - using GenericParallelDv01
// mod vanna; // removed - using GenericFdVanna
// mod volga; // removed - using GenericFdVolga
mod rho;
mod vega;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register lookback option metrics with the registry.
pub fn register_lookback_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{GenericFdDelta, GenericFdGamma, GenericFdVanna, GenericFdVolga};
    use crate::pricer::InstrumentType;

    // Use generic FD calculators for Delta, Gamma, Vanna, and Volga
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<crate::instruments::LookbackOption>::default()),
        &[InstrumentType::LookbackOption],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::LookbackOption>::default()),
        &[InstrumentType::LookbackOption],
    );

    registry.register_metric(
        MetricId::Vanna,
        Arc::new(GenericFdVanna::<crate::instruments::LookbackOption>::default()),
        &[InstrumentType::LookbackOption],
    );

    registry.register_metric(
        MetricId::Volga,
        Arc::new(GenericFdVolga::<crate::instruments::LookbackOption>::default()),
        &[InstrumentType::LookbackOption],
    );

    // Other metrics use custom implementations
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::LookbackOption,
        metrics: [
            (Vega, vega::VegaCalculator::default()),
            (Rho, rho::RhoCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::lookback_option::LookbackOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::lookback_option::LookbackOption,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
            // Theta is now registered universally in metrics::standard_registry()
        ]
    }
}
