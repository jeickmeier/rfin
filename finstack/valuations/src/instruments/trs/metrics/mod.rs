//! Risk metrics for Total Return Swaps.

mod annuity;
mod bucketed;
mod delta;
mod dividend_risk;
mod ir01;
mod par_spread;
mod theta;
// risk_bucketed_dv01 - now using generic implementation

pub use annuity::FinancingAnnuityCalculator;
pub use bucketed::TrsBucketedDv01Calculator;
pub use delta::IndexDeltaCalculator;
pub use ir01::TrsIR01Calculator;
pub use par_spread::ParSpreadCalculator;
// BucketedDv01Calculator now using generic implementation

use crate::metrics::MetricRegistry;

/// Registers all TRS metrics with the metric registry.
///
/// This function adds all TRS-specific metrics to the provided metric registry.
/// Each metric is registered for both EquityTotalReturnSwap and FIIndexTotalReturnSwap
/// instrument types.
///
/// # Arguments
/// * `registry` — Metric registry to add TRS metrics to
pub fn register_trs_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metric: Dividend risk (dividend yield sensitivity per 1bp)
    registry.register_metric(
        MetricId::custom("dividend01"),
        Arc::new(dividend_risk::DividendRiskCalculator),
        &["TRS"],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: "TRS",
        metrics: [
            (ParSpread, ParSpreadCalculator),
            (FinancingAnnuity, FinancingAnnuityCalculator),
            (Ir01, TrsIR01Calculator),
            (IndexDelta, IndexDeltaCalculator),
            (Theta, theta::EquityTrsThetaCalculator),
            (BucketedDv01, TrsBucketedDv01Calculator),
        ]
    }
}
