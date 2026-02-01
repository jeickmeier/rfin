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
// risk_bucketed_dv01 and dv01 - now using generic implementation
mod theta;
mod vega;

use crate::metrics::MetricRegistry;

/// Register all InterestRateOption metrics with the registry
pub fn register_interest_rate_option_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::CapFloor,
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Gamma, gamma::GammaCalculator),
            (Vega, vega::VegaCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::rates::cap_floor::InterestRateOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (Theta, theta::ThetaCalculator),
            (Rho, rho::RhoCalculator),
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
            (ForwardPv01, forward_pv01::ForwardPv01Calculator),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::rates::cap_floor::InterestRateOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }
}
