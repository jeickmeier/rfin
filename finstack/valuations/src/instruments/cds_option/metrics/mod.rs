//! CDS Option metrics module.
//!
//! Provides metric calculators specific to `CdsOption`, split into focused
//! files. The calculators compose with the shared metrics framework and are
//! registered via `register_cds_option_metrics`.
//!
//! Exposed metrics:
//! - Delta, Gamma, Vega, Theta, Rho
//! - Implied Volatility (placeholder)

mod delta;
mod gamma;
mod implied_vol;
mod rho;
mod risk_bucketed_dv01;
mod theta;
mod vega;

use crate::metrics::MetricRegistry;

/// Register all CDS Option metrics with the registry
pub fn register_cds_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry.register_metric(
        MetricId::Delta,
        Arc::new(delta::DeltaCalculator),
        &["CdsOption"],
    );
    registry.register_metric(
        MetricId::Gamma,
        Arc::new(gamma::GammaCalculator),
        &["CdsOption"],
    );
    registry.register_metric(
        MetricId::Vega,
        Arc::new(vega::VegaCalculator),
        &["CdsOption"],
    );
    registry.register_metric(
        MetricId::Theta,
        Arc::new(theta::ThetaCalculator),
        &["CdsOption"],
    );
    registry.register_metric(MetricId::Rho, Arc::new(rho::RhoCalculator), &["CdsOption"]);
    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(implied_vol::ImpliedVolCalculator),
        &["CdsOption"],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(risk_bucketed_dv01::BucketedDv01Calculator),
        &["CdsOption"],
    );
}
