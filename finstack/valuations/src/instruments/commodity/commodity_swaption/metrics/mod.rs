//! Commodity swaption metrics module.
//!
//! Registers option greeks and rate sensitivities for commodity swaptions.
//! Uses the generic option metric calculators from the shared metrics framework.

use crate::metrics::MetricRegistry;
use crate::pricer::InstrumentType;
use std::sync::Arc;

/// Register commodity swaption metrics with the registry.
pub fn register_commodity_swaption_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;

    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::CommoditySwaption,
        metrics: [
            (Delta, crate::metrics::OptionDeltaCalculator::<crate::instruments::CommoditySwaption>::default()),
            (Gamma, crate::metrics::OptionGammaCalculator::<crate::instruments::CommoditySwaption>::default()),
            (Vega, crate::metrics::OptionVegaCalculator::<crate::instruments::CommoditySwaption>::default()),
            (Theta, crate::metrics::GenericTheta::<crate::instruments::CommoditySwaption>::default()),
        ]
    }

    registry.register_metric(
        MetricId::Dv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::CommoditySwaption,
        >::new(
            crate::metrics::Dv01CalculatorConfig::parallel_combined()
        )),
        &[InstrumentType::CommoditySwaption],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::CommoditySwaption,
        >::new(
            crate::metrics::Dv01CalculatorConfig::triangular_key_rate(),
        )),
        &[InstrumentType::CommoditySwaption],
    );
}
