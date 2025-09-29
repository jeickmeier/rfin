//! Risk metrics for Total Return Swaps.

mod annuity;
mod bucketed;
mod delta;
mod ir01;
mod par_spread;
// risk_bucketed_dv01 - now using generic implementation

pub use annuity::FinancingAnnuityCalculator;
pub use bucketed::TrsBucketedDv01Calculator;
pub use delta::IndexDeltaCalculator;
pub use ir01::TrsIR01Calculator;
pub use par_spread::ParSpreadCalculator;
// BucketedDv01Calculator now using generic implementation

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

    registry.register_metric(MetricId::ParSpread, Arc::new(ParSpreadCalculator), &["TRS"]);
    registry.register_metric(
        MetricId::FinancingAnnuity,
        Arc::new(FinancingAnnuityCalculator),
        &["TRS"],
    );
    registry.register_metric(MetricId::Ir01, Arc::new(TrsIR01Calculator), &["TRS"]);
    registry.register_metric(
        MetricId::IndexDelta,
        Arc::new(IndexDeltaCalculator),
        &["TRS"],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(TrsBucketedDv01Calculator),
        &["TRS"],
    );
}
