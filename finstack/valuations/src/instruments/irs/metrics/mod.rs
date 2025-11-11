//! IRS metrics module.
//!
//! Exposes metric calculators for interest rate swaps, split into
//! focused files under this directory.

pub mod annuity;
pub mod convexity;
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
            (Dv01, crate::metrics::GenericParallelDv01::<
                crate::instruments::InterestRateSwap,
            >::default()),
            (IrConvexity, convexity::ConvexityCalculator),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::InterestRateSwap,
            >::default()),
            (BucketedDv01, crate::metrics::GenericBucketedDv01WithContext::<
                crate::instruments::InterestRateSwap,
            >::default()),
            (PvFixed, pv_fixed::FixedLegPvCalculator),
            (PvFloat, pv_float::FloatLegPvCalculator),
        ]
    }
}
