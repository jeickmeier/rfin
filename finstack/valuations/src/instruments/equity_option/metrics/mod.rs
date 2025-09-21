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

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register equity option metrics with the registry.
pub fn register_equity_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(delta::DeltaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(gamma::GammaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(
        MetricId::Vega,
        Arc::new(vega::VegaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(
        MetricId::Theta,
        Arc::new(theta::ThetaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(
        MetricId::Rho,
        Arc::new(rho::RhoCalculator),
        &["EquityOption"],
    );

    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(implied_vol::ImpliedVolCalculator),
        &["EquityOption"],
    );
}
