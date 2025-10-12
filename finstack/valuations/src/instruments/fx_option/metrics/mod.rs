//! FX option metrics module.
//!
//! Splits FX option metrics into focused calculators per greek and registers
//! them with the `MetricRegistry`. Calculators reuse the pricing engine
//! helpers to ensure consistency between PV and greeks.

mod delta;
mod gamma;
mod implied_vol;
mod rho;
mod risk_bucketed_dv01;
mod theta;
mod vega;

use crate::metrics::MetricRegistry;

/// Register FX option metrics with the registry.
pub fn register_fx_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metrics for rho split by domestic/foreign
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

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "FxOption",
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Gamma, gamma::GammaCalculator),
            (Vega, vega::VegaCalculator),
            (Theta, theta::ThetaCalculator),
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
            (BucketedDv01, risk_bucketed_dv01::BucketedDv01Calculator),
        ]
    }
}
