//! Interest rate option metrics module.
//!
//! Provides metric calculators specific to `InterestRateOption`, split into
//! focused files. The calculators compose with the shared metrics framework
//! and are registered via `register_interest_rate_option_metrics`.
//!
//! Exposed metrics:
//! - Delta
//! - Gamma
//! - Vega
//! - Theta
//! - Rho
//! - ImpliedVol (placeholder)

mod common;
mod delta;
mod forward_pv01;
mod gamma;
mod implied_vol;
mod rho;
mod theta;
mod vega;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register all InterestRateOption metrics with the registry
pub fn register_interest_rate_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(delta::DeltaCalculator),
        &["InterestRateOption"],
    );
    registry.register_metric(
        MetricId::Gamma,
        Arc::new(gamma::GammaCalculator),
        &["InterestRateOption"],
    );
    registry.register_metric(
        MetricId::Vega,
        Arc::new(vega::VegaCalculator),
        &["InterestRateOption"],
    );
    registry.register_metric(
        MetricId::Theta,
        Arc::new(theta::ThetaCalculator),
        &["InterestRateOption"],
    );
    registry.register_metric(
        MetricId::Rho,
        Arc::new(rho::RhoCalculator),
        &["InterestRateOption"],
    );
    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(implied_vol::ImpliedVolCalculator),
        &["InterestRateOption"],
    );
    registry.register_metric(
        MetricId::ForwardPv01,
        Arc::new(forward_pv01::ForwardPv01Calculator),
        &["InterestRateOption"],
    );
}
