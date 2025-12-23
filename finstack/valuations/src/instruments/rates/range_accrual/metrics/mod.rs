//! Range accrual metrics module.
//!
//! Provides full greek coverage for range accrual instruments using
//! finite difference methods. Delta and Gamma use generic FD calculators.
//! Includes bucketed DV01 for detailed interest rate risk analysis.

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

/// Register range accrual metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_range_accrual_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{GenericFdDelta, GenericFdGamma, GenericFdVanna, GenericFdVolga};
    use crate::pricer::InstrumentType;

    // Use generic FD calculators for Delta, Gamma, Vanna, and Volga
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<crate::instruments::RangeAccrual>::default()),
        &[InstrumentType::RangeAccrual],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::RangeAccrual>::default()),
        &[InstrumentType::RangeAccrual],
    );

    registry.register_metric(
        MetricId::Vanna,
        Arc::new(GenericFdVanna::<crate::instruments::RangeAccrual>::default()),
        &[InstrumentType::RangeAccrual],
    );

    registry.register_metric(
        MetricId::Volga,
        Arc::new(GenericFdVolga::<crate::instruments::RangeAccrual>::default()),
        &[InstrumentType::RangeAccrual],
    );

    // Other metrics use custom implementations
    #[cfg(feature = "mc")]
    {
        crate::register_metrics! {
            registry: registry,
            instrument: InstrumentType::RangeAccrual,
            metrics: [
                (Vega, vega::VegaCalculator::default()),
                (Rho, rho::RhoCalculator),
                (Dv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::range_accrual::RangeAccrual,
                >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
                // Theta is now registered universally in metrics::standard_registry()
                (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::RangeAccrual,
                >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
            ]
        }
    }
}
