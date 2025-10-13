//! Swaption metrics module.
//!
//! Contains per-metric calculators split into separate files for clarity and
//! maintainability. The module exposes a registration helper to wire metrics
//! into the shared `MetricRegistry`.

mod delta;
mod dv01;
mod gamma;
mod implied_vol;
mod rho;
// risk_bucketed_dv01 - now using generic implementation
mod theta;
mod vega;

pub use delta::DeltaCalculator;
pub use gamma::GammaCalculator;
pub use implied_vol::ImpliedVolCalculator;
pub use rho::RhoCalculator;
pub use theta::ThetaCalculator;
pub use vega::VegaCalculator;

use crate::metrics::MetricRegistry;

/// Register swaption metrics with the registry
pub fn register_swaption_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "Swaption",
        metrics: [
            (Delta, DeltaCalculator),
            (Gamma, GammaCalculator),
            (Vega, VegaCalculator),
            (Dv01, dv01::SwaptionDv01Calculator),
            (Theta, ThetaCalculator),
            (Rho, RhoCalculator),
            (ImpliedVol, ImpliedVolCalculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::Swaption,
            >::default()),
        ]
    }
}

/// Swaption metrics configuration constants.
/// Centralizes scaling parameters to avoid magic numbers in calculators.
pub mod config {

    /// Basis points per unit (1.0 = 1 bp; 10000.0 = 100%).
    pub const BP_PER_UNIT: f64 = 10000.0;
    /// Vega scale for 1% volatility change.
    pub const VOL_PCT_SCALE: f64 = 100.0;
    /// Trading days per year for theta scaling (daily).
    pub const THETA_DAYS_PER_YEAR: f64 = 365.0;
    /// Discount curve parallel bump size in basis points for rho.
    pub const DISC_BUMP_BP: f64 = 1.0;
    /// Multiplier to convert 1bp PV change to per 1% rate move.
    pub const RHO_PCT_PER_BP: f64 = 100.0;
}
