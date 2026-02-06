//! FX digital option metrics module.
//!
//! Registers option Greeks and risk metrics for FX digital options.

use crate::metrics::MetricRegistry;

/// Register FX digital option metrics with the registry.
pub fn register_fx_digital_option_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;

    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxDigitalOption,
        metrics: [
            (Delta, crate::metrics::OptionDeltaCalculator::<crate::instruments::FxDigitalOption>::default()),
            (Gamma, crate::metrics::OptionGammaCalculator::<crate::instruments::FxDigitalOption>::default()),
            (Vega, crate::metrics::OptionVegaCalculator::<crate::instruments::FxDigitalOption>::default()),
            (Theta, crate::metrics::OptionThetaCalculator::<crate::instruments::FxDigitalOption>::default()),
            (Rho, crate::metrics::OptionRhoCalculator::<crate::instruments::FxDigitalOption>::default()),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxDigitalOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxDigitalOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }
}
