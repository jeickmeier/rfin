//! Swaption metrics module.
//!
//! Contains per-metric calculators split into separate files for clarity and
//! maintainability. The module exposes a registration helper to wire metrics
//! into the shared `MetricRegistry`.

pub mod bermudan_greeks;
mod delta;
mod gamma;
mod implied_vol;
mod rho;
mod vega;

pub use bermudan_greeks::{
    BermudanDeltaCalculator, BermudanGammaCalculator, BermudanVegaCalculator,
    ExerciseProbabilityCalculator, ExerciseProbabilityProfile,
};
pub use delta::DeltaCalculator;
pub use gamma::GammaCalculator;
pub use implied_vol::ImpliedVolCalculator;
pub use rho::RhoCalculator;
pub use vega::VegaCalculator;

use crate::metrics::MetricRegistry;

/// Register swaption metrics with the registry
pub fn register_swaption_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::Swaption,
        metrics: [
            (Delta, DeltaCalculator),
            (Gamma, GammaCalculator),
            (Vega, VegaCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Swaption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            // Theta is now registered universally in metrics::standard_registry()
            (Rho, RhoCalculator),
            (ImpliedVol, ImpliedVolCalculator),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Swaption,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
        ]
    }
}

/// Register Bermudan swaption metrics with the registry
pub fn register_bermudan_swaption_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::BermudanSwaption,
        metrics: [
            (Delta, BermudanDeltaCalculator::new()),
            (Gamma, BermudanGammaCalculator::new()),
            (Vega, BermudanVegaCalculator::new()),
        ]
    }
    // Register custom ExerciseProbability metric separately
    registry.register_metric(
        crate::metrics::MetricId::custom("ExerciseProbability"),
        std::sync::Arc::new(ExerciseProbabilityCalculator::new()),
        &[InstrumentType::BermudanSwaption],
    );
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
