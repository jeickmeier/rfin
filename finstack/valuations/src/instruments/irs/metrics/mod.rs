//! IRS metrics module.
//!
//! Exposes metric calculators for interest rate swaps, split into
//! focused files under this directory.

pub mod annuity;
pub mod par_rate;
pub mod pv_fixed;
pub mod pv_float;
// risk_bucketed_dv01, dv01, and theta now using generic implementations

/// Registers all IRS metrics into a provided registry.
pub fn register_irs_metrics(registry: &mut crate::metrics::MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "InterestRateSwap",
        metrics: [
            (Annuity, annuity::AnnuityCalculator),
            (ParRate, par_rate::ParRateCalculator),

            // Theta is now registered universally in metrics::standard_registry()

            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InterestRateSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),

            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InterestRateSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
            
            (PvFixed, pv_fixed::FixedLegPvCalculator),
            (PvFloat, pv_float::FloatLegPvCalculator),
        ]
    }
}
