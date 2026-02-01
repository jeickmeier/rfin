//! Quanto option metrics module.
//!
//! Provides full greek coverage for quanto options including FX-specific
//! sensitivities (FX delta, FX vega) and correlation risk.

#[cfg(feature = "mc")]
mod correlation01;
// mod dv01; // removed - using GenericParallelDv01
#[cfg(feature = "mc")]
mod fx_delta;
#[cfg(feature = "mc")]
mod fx_vega;

#[cfg(feature = "mc")]
use crate::metrics::{MetricId, MetricRegistry};
#[cfg(feature = "mc")]
use std::sync::Arc;

/// Register quanto option metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_quanto_option_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    #[cfg(feature = "mc")]
    {
        // Standard greeks
        crate::register_metrics! {
            registry: registry,
            instrument: InstrumentType::QuantoOption,
            metrics: [
                (Delta, crate::metrics::OptionDeltaCalculator::<crate::instruments::fx::quanto_option::QuantoOption>::default()),
                (Gamma, crate::metrics::OptionGammaCalculator::<crate::instruments::fx::quanto_option::QuantoOption>::default()),
                (Vega, crate::metrics::OptionVegaCalculator::<crate::instruments::fx::quanto_option::QuantoOption>::default()),
                (Rho, crate::metrics::OptionRhoCalculator::<crate::instruments::fx::quanto_option::QuantoOption>::default()),
                (ForeignRho, crate::metrics::OptionForeignRhoCalculator::<crate::instruments::fx::quanto_option::QuantoOption>::default()),
                (Dv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::fx::quanto_option::QuantoOption,
                >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
                (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::fx::quanto_option::QuantoOption,
                >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
                (Vanna, crate::metrics::OptionVannaCalculator::<crate::instruments::fx::quanto_option::QuantoOption>::default()),
                (Volga, crate::metrics::OptionVolgaCalculator::<crate::instruments::fx::quanto_option::QuantoOption>::default()),
                // Theta is now registered universally in metrics::standard_registry()
            ]
        }

        // FX-specific and correlation metrics (using custom MetricIds)
        registry.register_metric(
            MetricId::custom("fx_delta"),
            Arc::new(fx_delta::FxDeltaCalculator),
            &[InstrumentType::QuantoOption],
        );

        registry.register_metric(
            MetricId::FxVega,
            Arc::new(fx_vega::FxVegaCalculator),
            &[InstrumentType::QuantoOption],
        );

        registry.register_metric(
            MetricId::Correlation01,
            Arc::new(correlation01::Correlation01Calculator),
            &[InstrumentType::QuantoOption],
        );
    }
}
