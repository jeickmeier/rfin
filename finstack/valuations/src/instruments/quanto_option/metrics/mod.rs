//! Quanto option metrics module.
//!
//! Provides full greek coverage for quanto options including FX-specific
//! sensitivities (FX delta, FX vega) and correlation risk.

#[cfg(feature = "mc")]
mod correlation01;
#[cfg(feature = "mc")]
mod delta;
#[cfg(feature = "mc")]
mod dv01;
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
    #[cfg(feature = "mc")]
    {
        // Standard greeks
        crate::register_metrics! {
            registry: registry,
            instrument: "QuantoOption",
            metrics: [
                (Delta, delta::DeltaCalculator),
                (Gamma, gamma::GammaCalculator),
                (Vega, vega::VegaCalculator),
                (Rho, rho::RhoCalculator),
                (Dv01, dv01::Dv01Calculator),
                (Vanna, vanna::VannaCalculator),
                (Volga, volga::VolgaCalculator),
                (Theta, crate::instruments::common::metrics::GenericTheta::<
                    crate::instruments::QuantoOption,
                >::default()),
            ]
        }

        // FX-specific and correlation metrics (using custom MetricIds)
        registry.register_metric(
            MetricId::custom("fx_delta"),
            Arc::new(fx_delta::FxDeltaCalculator),
            &["QuantoOption"],
        );

        registry.register_metric(
            MetricId::FxVega,
            Arc::new(fx_vega::FxVegaCalculator),
            &["QuantoOption"],
        );

        registry.register_metric(
            MetricId::Correlation01,
            Arc::new(correlation01::Correlation01Calculator),
            &["QuantoOption"],
        );
    }
}
