//! Swaption metrics module.
//!
//! Contains per-metric calculators split into separate files for clarity and
//! maintainability. The module exposes a registration helper to wire metrics
//! into the shared `MetricRegistry`.

mod delta;
mod gamma;
mod vega;
mod theta;
mod rho;
mod implied_vol;

pub use delta::DeltaCalculator;
pub use gamma::GammaCalculator;
pub use vega::VegaCalculator;
pub use theta::ThetaCalculator;
pub use rho::RhoCalculator;
pub use implied_vol::ImpliedVolCalculator;

use crate::metrics::{MetricId, MetricRegistry};
use finstack_core::F;
use std::sync::Arc;

/// Register swaption metrics with the registry
pub fn register_swaption_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(MetricId::Delta, Arc::new(DeltaCalculator), &["Swaption"]);
    registry.register_metric(MetricId::Gamma, Arc::new(GammaCalculator), &["Swaption"]);
    registry.register_metric(MetricId::Vega, Arc::new(VegaCalculator), &["Swaption"]);
    registry.register_metric(MetricId::Theta, Arc::new(ThetaCalculator), &["Swaption"]);
    registry.register_metric(MetricId::Rho, Arc::new(RhoCalculator), &["Swaption"]);
    registry.register_metric(MetricId::ImpliedVol, Arc::new(ImpliedVolCalculator), &["Swaption"]);
}

/// Swaption metrics configuration constants.
/// Centralizes scaling parameters to avoid magic numbers in calculators.
pub mod config {
    use super::F;

    /// Basis points per unit (1.0 = 1 bp; 10000.0 = 100%).
    pub const BP_PER_UNIT: F = 10000.0;
    /// Vega scale for 1% volatility change.
    pub const VOL_PCT_SCALE: F = 100.0;
    /// Trading days per year for theta scaling (daily).
    pub const THETA_DAYS_PER_YEAR: F = 365.0;
    /// Discount curve parallel bump size in basis points for rho.
    pub const DISC_BUMP_BP: F = 1.0;
    /// Multiplier to convert 1bp PV change to per 1% rate move.
    pub const RHO_PCT_PER_BP: F = 100.0;
}
