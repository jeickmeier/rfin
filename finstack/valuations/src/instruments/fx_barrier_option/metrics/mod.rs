//! FX barrier option metrics module.
//!
//! Provides full greek coverage for FX barrier options using finite difference methods.
//! Note: FX barrier options exhibit discontinuous greeks near the barrier level.
//! Delta represents FX spot sensitivity.

#[cfg(feature = "mc")]
mod delta;
#[cfg(feature = "mc")]
mod dv01;
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

use crate::metrics::MetricRegistry;

/// Register FX barrier option metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_fx_barrier_option_metrics(registry: &mut MetricRegistry) {
    #[cfg(feature = "mc")]
    {
        crate::register_metrics! {
            registry: registry,
            instrument: "FxBarrierOption",
            metrics: [
                (Delta, delta::DeltaCalculator),
                (Gamma, gamma::GammaCalculator),
                (Vega, vega::VegaCalculator),
                (Rho, rho::RhoCalculator),
                (Dv01, dv01::Dv01Calculator),
                (Vanna, vanna::VannaCalculator),
                (Volga, volga::VolgaCalculator),
                (Theta, crate::metrics::GenericTheta::<
                    crate::instruments::FxBarrierOption,
                >::default()),
            ]
        }
    }
}

/// Register FX barrier option metrics when MC feature is not available.
#[cfg(not(feature = "mc"))]
pub fn register_fx_barrier_option_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "FxBarrierOption",
        metrics: [
            (Dv01, crate::metrics::GenericParallelDv01::<
                crate::instruments::FxBarrierOption,
            >::default()),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::FxBarrierOption,
            >::default()),
        ]
    }
}
