//! Quanto option metrics module.
//!
//! Provides full greek coverage for quanto options including FX-specific
//! sensitivities (FX delta, FX vega) and correlation risk.

#[cfg(feature = "mc")]
mod correlation01;
#[cfg(feature = "mc")]
mod delta;
// mod dv01; // removed - using GenericParallelDv01
#[cfg(feature = "mc")]
mod foreign_rho;
#[cfg(feature = "mc")]
mod fx_delta;
#[cfg(feature = "mc")]
mod fx_vega;
#[cfg(feature = "mc")]
mod gamma;
#[cfg(feature = "mc")]
mod rho;
#[cfg(feature = "mc")]
mod vanna;
#[cfg(feature = "mc")]
mod vega;
#[cfg(feature = "mc")]
mod volga;

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
                (Delta, delta::DeltaCalculator),
                (Gamma, gamma::GammaCalculator),
                (Vega, vega::VegaCalculator),
                (Rho, rho::RhoCalculator),
                (ForeignRho, foreign_rho::ForeignRhoCalculator),
                (Dv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::quanto_option::QuantoOption,
                >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
                (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                    crate::instruments::quanto_option::QuantoOption,
                >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
                (Vanna, vanna::VannaCalculator),
                (Volga, volga::VolgaCalculator),
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
