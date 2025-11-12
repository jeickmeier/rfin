//! Cliquet option metrics module.
//!
//! Provides full greek coverage for cliquet options using finite difference methods.
//! Delta and Gamma use generic FD calculators.

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

/// Register cliquet option metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_cliquet_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{GenericFdDelta, GenericFdGamma};

    // Use generic FD calculators for Delta and Gamma
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<crate::instruments::CliquetOption>::default()),
        &["CliquetOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::CliquetOption>::default()),
        &["CliquetOption"],
    );

    // Other metrics use custom implementations
    #[cfg(feature = "mc")]
    {
        crate::register_metrics! {
            registry: registry,
            instrument: "CliquetOption",
            metrics: [
                (Vega, vega::VegaCalculator::default()),
                (Rho, rho::RhoCalculator),
                (Dv01, crate::metrics::GenericParallelDv01::<
                    crate::instruments::CliquetOption,
                >::default()),
                (BucketedDv01, crate::metrics::GenericBucketedDv01WithContext::<
                    crate::instruments::CliquetOption,
                >::default()),
                (Vanna, vanna::VannaCalculator),
                (Volga, volga::VolgaCalculator::default()),
                (Theta, crate::metrics::GenericTheta::<
                    crate::instruments::CliquetOption,
                >::default()),
            ]
        }
    }
}
