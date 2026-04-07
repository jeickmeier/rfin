//! FX barrier option metrics module.
//!
//! Provides full greek coverage for FX barrier options using finite difference methods.
//! Note: FX barrier options exhibit discontinuous greeks near the barrier level.
//! Delta represents FX spot sensitivity.

use crate::metrics::MetricRegistry;

/// Register FX barrier option metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_fx_barrier_option_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxBarrierOption,
        metrics: [
            (Delta, crate::metrics::OptionDeltaCalculator::<crate::instruments::FxBarrierOption>::default()),
            (Gamma, crate::metrics::OptionGammaCalculator::<crate::instruments::FxBarrierOption>::default()),
            (Vega, crate::metrics::OptionVegaCalculator::<crate::instruments::FxBarrierOption>::default()),
            (Rho, crate::metrics::OptionRhoCalculator::<crate::instruments::FxBarrierOption>::default()),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxBarrierOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxBarrierOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Vanna, crate::metrics::OptionVannaCalculator::<crate::instruments::FxBarrierOption>::default()),
            (Volga, crate::metrics::OptionVolgaCalculator::<crate::instruments::FxBarrierOption>::default()),
        ]
    }
}

/// Register FX barrier option metrics when MC feature is not available.
#[cfg(not(feature = "mc"))]
pub(crate) fn register_fx_barrier_option_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxBarrierOption,
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxBarrierOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxBarrierOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }
}
