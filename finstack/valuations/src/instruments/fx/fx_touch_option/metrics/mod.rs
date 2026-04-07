//! FX touch option metrics module.
//!
//! Provides risk metrics for FX touch options using finite difference methods.
//! Touch options exhibit discontinuous Greeks near the barrier, so finite
//! differences are preferred over analytical formulas.

use crate::metrics::MetricRegistry;

/// Register FX touch option metrics with the registry.
pub(crate) fn register_fx_touch_option_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;

    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxTouchOption,
        metrics: [
            (Delta, crate::metrics::OptionDeltaCalculator::<crate::instruments::FxTouchOption>::default()),
            (Gamma, crate::metrics::OptionGammaCalculator::<crate::instruments::FxTouchOption>::default()),
            (Vega, crate::metrics::OptionVegaCalculator::<crate::instruments::FxTouchOption>::default()),
            (Rho, crate::metrics::OptionRhoCalculator::<crate::instruments::FxTouchOption>::default()),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxTouchOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxTouchOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }
}
