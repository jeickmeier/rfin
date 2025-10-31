//! Quanto option metrics module.
//!
//! Provides full greek coverage for quanto options including FX-specific
//! sensitivities (FX delta, FX vega) and correlation risk.

mod correlation01;
mod delta;
mod dv01;
mod fx_delta;
mod fx_vega;
mod gamma;
mod rho;
mod vanna;
mod vega;
mod volga;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register quanto option metrics with the registry.
pub fn register_quanto_option_metrics(registry: &mut MetricRegistry) {
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
