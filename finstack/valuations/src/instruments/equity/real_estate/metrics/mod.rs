//! Real estate asset metrics module.
//!
//! Provides standard rate risk metrics for real estate valuations.

use crate::metrics::MetricRegistry;

/// Register real estate asset metrics with the registry.
pub fn register_real_estate_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::RealEstateAsset,
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::RealEstateAsset,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::RealEstateAsset,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::RealEstateAsset,
            >::default()),
        ]
    };
}
