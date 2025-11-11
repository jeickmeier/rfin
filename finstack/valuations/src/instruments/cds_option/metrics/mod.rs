//! CDS Option metrics module.
//!
//! Provides metric calculators specific to `CdsOption`, split into focused
//! files. The calculators compose with the shared metrics framework and are
//! registered via `register_cds_option_metrics`.
//!
//! Exposed metrics:
//! - Delta, Gamma, Vega, Theta, Rho
//! - CS01 (credit spread sensitivity)
//! - Implied Volatility (placeholder)

mod cs01;
mod delta;
mod dv01;
mod gamma;
mod implied_vol;
mod recovery01;
mod rho;
// risk_bucketed_dv01 - now using generic implementation
mod vega;

use crate::metrics::MetricRegistry;

/// Register all CDS Option metrics with the registry
pub fn register_cds_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Recovery01 (custom metric - recovery rate sensitivity)
    registry.register_metric(
        MetricId::Recovery01,
        Arc::new(recovery01::Recovery01Calculator),
        &["CDSOption"],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: "CDSOption",
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Gamma, gamma::GammaCalculator),
            (Vega, vega::VegaCalculator),
            (Cs01, cs01::Cs01Calculator),
            (Dv01, dv01::CdsOptionDv01Calculator),
            // Theta is now registered universally in metrics::standard_registry()
            (Rho, rho::RhoCalculator),
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
            (BucketedDv01, crate::metrics::GenericBucketedDv01WithContext::<
                crate::instruments::CdsOption,
            >::default()),
        ]
    }
}
