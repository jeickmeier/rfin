//! XCCY swap metrics module.
//!
//! Registers standard rate risk metrics for cross-currency swaps.

use crate::metrics::MetricRegistry;

/// Register XCCY swap metrics with the registry.
pub fn register_xccy_swap_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::sensitivities::cross_factor::{
        CrossFactorCalculator, CrossFactorPair, FxBumperFactory, RatesBumperFactory,
    };
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    registry.register_metric(
        MetricId::CrossGammaFxRates,
        Arc::new(CrossFactorCalculator::new(
            CrossFactorPair::FxRates,
            Arc::new(FxBumperFactory),
            Arc::new(RatesBumperFactory),
        )),
        &[InstrumentType::XccySwap],
    );

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
        ]
    };
}
