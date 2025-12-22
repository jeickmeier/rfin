//! Risk metrics for Fixed Income Index Total Return Swaps.
//!
//! This module provides FI TRS-specific risk metrics:
//! - **DurationDelta**: Sensitivity to index level (duration-weighted)
//! - **ParSpread**: Spread that makes NPV = 0
//! - **FinancingAnnuity**: PV01 of the financing leg

mod annuity;
mod duration_delta;
mod par_spread;

pub use annuity::FinancingAnnuityCalculator;
pub use duration_delta::DurationDeltaCalculator;
pub use par_spread::ParSpreadCalculator;

use crate::metrics::MetricRegistry;
use crate::pricer::InstrumentType;

/// Registers all fixed income TRS metrics with the metric registry.
///
/// # Arguments
/// * `registry` — Metric registry to add FI TRS metrics to
pub fn register_fi_trs_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    let instruments = [InstrumentType::FIIndexTotalReturnSwap];

    // FI TRS specific metrics
    registry.register_metric(
        MetricId::IndexDelta,
        Arc::new(DurationDeltaCalculator),
        &instruments,
    );
    registry.register_metric(
        MetricId::ParSpread,
        Arc::new(ParSpreadCalculator),
        &instruments,
    );
    registry.register_metric(
        MetricId::FinancingAnnuity,
        Arc::new(FinancingAnnuityCalculator),
        &instruments,
    );

    // DV01 for financing leg sensitivity
    registry.register_metric(
        MetricId::Dv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::fi_trs::FIIndexTotalReturnSwap,
        >::new(
            crate::metrics::Dv01CalculatorConfig::parallel_combined()
        )),
        &instruments,
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::fi_trs::FIIndexTotalReturnSwap,
        >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
        &instruments,
    );
}

