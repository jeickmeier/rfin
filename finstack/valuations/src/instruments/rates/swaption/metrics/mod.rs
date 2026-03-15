//! Swaption metrics module.
//!
//! Contains per-metric calculators split into separate files for clarity and
//! maintainability. The module exposes a registration helper to wire metrics
//! into the shared `MetricRegistry`.

#![allow(dead_code)] // WIP: public API not yet wired into main pricing paths

pub mod bermudan_greeks;
mod delta;
mod gamma;
mod implied_vol;
mod vega;

pub use bermudan_greeks::{
    BermudanDeltaCalculator, BermudanGammaCalculator, BermudanVegaCalculator,
    ExerciseProbabilityCalculator,
};
pub use delta::DeltaCalculator;
pub use gamma::GammaCalculator;
pub use implied_vol::ImpliedVolCalculator;
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
            (Rho, crate::metrics::GenericRho::<crate::instruments::Swaption>::default()),
            (ImpliedVol, ImpliedVolCalculator),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::Swaption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }
}

/// Register Bermudan swaption metrics with the registry.
///
/// # Arguments
///
/// * `registry` - The metric registry to register calculators with
/// * `hw_params` - Calibrated Hull-White parameters for the Bermudan Greek calculators.
///   These should be calibrated to co-terminal European swaption prices from
///   the volatility surface. Using uncalibrated defaults can produce Bermudan
///   premium errors of 10-30% of the early exercise value.
///
/// # Important
///
/// `BermudanSwaption::value()` is not implemented (it requires a tree/LSMC pricer),
/// so generic calculators like `UnifiedDv01Calculator` and `GenericRho` that rely on
/// `Instrument::value()` cannot be used. Only bump-and-revalue calculators that use
/// the explicit tree pricer are registered.
///
/// # Example
///
/// ```rust,ignore
/// use finstack_valuations::instruments::rates::swaption::HullWhiteParams;
///
/// // Calibrate parameters to your vol surface first
/// let calibrated_params = HullWhiteParams::new(0.05, 0.008);
///
/// let mut registry = MetricRegistry::new();
/// register_bermudan_swaption_metrics(&mut registry, calibrated_params);
/// ```
#[allow(dead_code)] // May be used by external bindings or tests
pub fn register_bermudan_swaption_metrics(
    registry: &mut MetricRegistry,
    hw_params: crate::instruments::rates::swaption::HullWhiteParams,
) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::BermudanSwaption,
        metrics: [
            (Delta, BermudanDeltaCalculator::new()
                .with_hw_params(hw_params.kappa, hw_params.sigma)),
            (Gamma, BermudanGammaCalculator::new()
                .with_hw_params(hw_params.kappa, hw_params.sigma)),
            (Vega, BermudanVegaCalculator::new()
                .with_hw_params(hw_params.kappa, hw_params.sigma))
            // Note: UnifiedDv01Calculator, GenericRho, and BucketedDv01 are NOT
            // registered here because BermudanSwaption::value() returns Err.
            // Use BermudanDeltaCalculator for rate sensitivity instead.
            // Theta is registered universally in metrics::standard_registry()
            // but will fail for BermudanSwaption for the same reason.
        ]
    }
    // Register custom ExerciseProbability metric separately
    registry.register_metric(
        crate::metrics::MetricId::custom("ExerciseProbability"),
        std::sync::Arc::new(ExerciseProbabilityCalculator::new_with_hw(
            hw_params.kappa,
            hw_params.sigma,
        )),
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
