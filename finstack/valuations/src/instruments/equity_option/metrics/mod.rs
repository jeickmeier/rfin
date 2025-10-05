//! Equity option metrics module.
//!
//! Splits equity option metrics into focused calculators per greek and
//! registers them with the `MetricRegistry`. Calculators reuse the pricing
//! engine helpers to ensure consistency between PV and greeks.

mod delta;
mod gamma;
mod implied_vol;
mod rho;
mod theta;
mod vega;

use crate::metrics::MetricRegistry;

/// Register equity option metrics with the registry.
pub fn register_equity_option_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "EquityOption",
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Gamma, gamma::GammaCalculator),
            (Vega, vega::VegaCalculator),
            (Theta, theta::ThetaCalculator),
            (Rho, rho::RhoCalculator),
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
        ]
    }
}
