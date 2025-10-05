//! IRS metrics module.
//!
//! Exposes metric calculators for interest rate swaps, split into
//! focused files under this directory.

pub mod annuity;
pub mod dv01;
pub mod par_rate;
pub mod pv_fixed;
pub mod pv_float;
// risk_bucketed_dv01 - now using generic implementation

/// Registers all IRS metrics into a provided registry.
pub fn register_irs_metrics(registry: &mut crate::metrics::MetricRegistry) {
    crate::register_metrics_chained! {
        registry: registry,
        instrument: "InterestRateSwap",
        metrics: [
            (Annuity, annuity::AnnuityCalculator),
            (ParRate, par_rate::ParRateCalculator),
            (Dv01, dv01::Dv01Calculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::InterestRateSwap,
            >::default()),
            (PvFixed, pv_fixed::FixedLegPvCalculator),
            (PvFloat, pv_float::FloatLegPvCalculator),
        ]
    };
}
