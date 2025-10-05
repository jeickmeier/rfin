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
// risk_bucketed_dv01 - now using generic implementation
mod theta;
mod vega;

use crate::metrics::MetricRegistry;

/// Register all CDS Option metrics with the registry
pub fn register_cds_option_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "CdsOption",
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Gamma, gamma::GammaCalculator),
            (Vega, vega::VegaCalculator),
            (Theta, theta::ThetaCalculator),
            (Rho, rho::RhoCalculator),
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::CdsOption,
            >::default()),
        ]
    }
}
