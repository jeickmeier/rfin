//! Risk metrics for Total Return Swaps.

mod annuity;
mod delta;
mod ir01;
mod par_spread;
mod risk_bucketed_dv01;

pub use annuity::FinancingAnnuityCalculator;
pub use delta::IndexDeltaCalculator;
pub use ir01::TrsIR01Calculator;
pub use par_spread::ParSpreadCalculator;
pub use risk_bucketed_dv01::BucketedDv01Calculator;

use crate::metrics::{MetricId, MetricRegistry};

/// Registers all TRS metrics with the metric registry.
///
/// This function adds all TRS-specific metrics to the provided metric registry.
/// Each metric is registered for both EquityTotalReturnSwap and FIIndexTotalReturnSwap
/// instrument types.
///
/// # Arguments
/// * `registry` — Metric registry to add TRS metrics to
pub fn register_trs_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;

    registry.register_metric(
        MetricId::ParSpread,
        Arc::new(ParSpreadCalculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
    registry.register_metric(
        MetricId::FinancingAnnuity,
        Arc::new(FinancingAnnuityCalculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
    registry.register_metric(
        MetricId::Ir01,
        Arc::new(TrsIR01Calculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
    registry.register_metric(
        MetricId::IndexDelta,
        Arc::new(IndexDeltaCalculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(BucketedDv01Calculator),
        &["EquityTotalReturnSwap", "FIIndexTotalReturnSwap"],
    );
}
