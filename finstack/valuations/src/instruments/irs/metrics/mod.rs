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
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::Annuity,
            Arc::new(annuity::AnnuityCalculator),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::ParRate,
            Arc::new(par_rate::ParRateCalculator),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::Dv01,
            Arc::new(dv01::Dv01Calculator),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(crate::instruments::common::GenericBucketedDv01WithContext::<crate::instruments::InterestRateSwap>::default()),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::PvFixed,
            Arc::new(pv_fixed::FixedLegPvCalculator),
            &["InterestRateSwap"],
        )
        .register_metric(
            MetricId::PvFloat,
            Arc::new(pv_float::FloatLegPvCalculator),
            &["InterestRateSwap"],
        );
}
