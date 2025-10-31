//! Cliquet option metrics module.
//!
//! Provides full greek coverage for cliquet options using finite difference methods.
//! Delta and Gamma use generic FD calculators.

mod dv01;
mod rho;
mod vanna;
mod vega;
mod volga;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register cliquet option metrics with the registry.
pub fn register_cliquet_option_metrics(registry: &mut MetricRegistry) {
    use crate::instruments::common::metrics::{GenericFdDelta, GenericFdGamma};

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
    crate::register_metrics! {
        registry: registry,
        instrument: "CliquetOption",
        metrics: [
            (Vega, vega::VegaCalculator),
            (Rho, rho::RhoCalculator),
            (Dv01, dv01::Dv01Calculator),
            (Vanna, vanna::VannaCalculator),
            (Volga, volga::VolgaCalculator),
            (Theta, crate::instruments::common::metrics::GenericTheta::<
                crate::instruments::CliquetOption,
            >::default()),
        ]
    }
}
