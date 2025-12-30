//! XCCY swap metrics module.
//!
//! Registers standard rate risk metrics for cross-currency swaps.

use crate::metrics::MetricRegistry;

/// Register XCCY swap metrics with the registry.
pub fn register_xccy_swap_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::XccySwap,
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::XccySwap,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::XccySwap,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::XccySwap,
            >::default()),
        ]
    };
}
