//! IRS metrics module facade.
//!
//! Exposes metric calculators for interest rate swaps, split into
//! focused files under this directory. This shim keeps external imports
//! stable while enabling maintainable growth of metrics.

pub mod annuity;
pub mod dv01;
pub mod par_rate;
pub mod pv_fixed;
pub mod pv_float;
pub mod risk_bucketed_dv01;

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
            Arc::new(risk_bucketed_dv01::BucketedDv01Calculator),
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
