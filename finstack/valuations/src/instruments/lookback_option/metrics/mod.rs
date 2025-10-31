//! Lookback option metrics module.
//!
//! Provides full greek coverage for lookback options using finite difference methods.
//! Delta and Gamma use generic FD calculators.

#[cfg(feature = "mc")]
mod dv01;
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

/// Register lookback option metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_lookback_option_metrics(registry: &mut MetricRegistry) {
    use crate::instruments::common::metrics::{GenericFdDelta, GenericFdGamma};

    // Use generic FD calculators for Delta and Gamma
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<crate::instruments::LookbackOption>::default()),
        &["LookbackOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::LookbackOption>::default()),
        &["LookbackOption"],
    );

    // Other metrics use custom implementations
    #[cfg(feature = "mc")]
    {
        crate::register_metrics! {
            registry: registry,
            instrument: "LookbackOption",
            metrics: [
                (Vega, vega::VegaCalculator),
                (Rho, rho::RhoCalculator),
                (Dv01, dv01::Dv01Calculator),
                (Vanna, vanna::VannaCalculator),
                (Volga, volga::VolgaCalculator),
                (Theta, crate::instruments::common::metrics::GenericTheta::<
                    crate::instruments::LookbackOption,
                >::default()),
            ]
        }
    }
}
