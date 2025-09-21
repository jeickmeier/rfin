//! FX option metrics module.
//!
//! Splits FX option metrics into focused calculators per greek and registers
//! them with the `MetricRegistry`. Calculators reuse the pricing engine
//! helpers to ensure consistency between PV and greeks.

mod delta;
mod gamma;
mod implied_vol;
mod rho;
mod theta;
mod vega;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register FX option metrics with the registry.
pub fn register_fx_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(delta::DeltaCalculator),
        &["FxOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(gamma::GammaCalculator),
        &["FxOption"],
    );

    registry.register_metric(
        MetricId::Vega,
        Arc::new(vega::VegaCalculator),
        &["FxOption"],
    );

    registry.register_metric(
        MetricId::Theta,
        Arc::new(theta::ThetaCalculator),
        &["FxOption"],
    );

    registry.register_metric(
        MetricId::custom("rho_domestic"),
        Arc::new(rho::RhoDomesticCalculator),
        &["FxOption"],
    );

    registry.register_metric(
        MetricId::custom("rho_foreign"),
        Arc::new(rho::RhoForeignCalculator),
        &["FxOption"],
    );

    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(implied_vol::ImpliedVolCalculator),
        &["FxOption"],
    );
}
