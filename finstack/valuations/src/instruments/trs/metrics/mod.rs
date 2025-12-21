//! Risk metrics for Total Return Swaps.

mod annuity;
mod delta;
mod dividend_risk;
mod par_spread;

pub use annuity::FinancingAnnuityCalculator;
pub use delta::IndexDeltaCalculator;
pub use par_spread::ParSpreadCalculator;

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
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    let instruments = [
        InstrumentType::EquityTotalReturnSwap,
        InstrumentType::FIIndexTotalReturnSwap,
    ];

    // Common metrics that handle runtime dispatch internally
    let common_metrics = vec![
        (
            MetricId::Dividend01,
            Arc::new(dividend_risk::DividendRiskCalculator)
                as Arc<dyn crate::metrics::MetricCalculator>,
        ),
        (
            MetricId::ParSpread,
            Arc::new(ParSpreadCalculator) as Arc<dyn crate::metrics::MetricCalculator>,
        ),
        (
            MetricId::FinancingAnnuity,
            Arc::new(FinancingAnnuityCalculator) as Arc<dyn crate::metrics::MetricCalculator>,
        ),
        (
            MetricId::IndexDelta,
            Arc::new(IndexDeltaCalculator) as Arc<dyn crate::metrics::MetricCalculator>,
        ),
    ];

    for (metric_id, calculator) in common_metrics {
        registry.register_metric(metric_id, calculator, &instruments);
    }

    // Equity TRS Specifics
    registry.register_metric(
        MetricId::Dv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::trs::EquityTotalReturnSwap,
        >::new(
            crate::metrics::Dv01CalculatorConfig::parallel_combined()
        )),
        &[InstrumentType::EquityTotalReturnSwap],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::trs::EquityTotalReturnSwap,
        >::new(
            crate::metrics::Dv01CalculatorConfig::key_rate()
        )),
        &[InstrumentType::EquityTotalReturnSwap],
    );

    // FI Index TRS Specifics
    registry.register_metric(
        MetricId::Dv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::trs::FIIndexTotalReturnSwap,
        >::new(
            crate::metrics::Dv01CalculatorConfig::parallel_combined()
        )),
        &[InstrumentType::FIIndexTotalReturnSwap],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::trs::FIIndexTotalReturnSwap,
        >::new(
            crate::metrics::Dv01CalculatorConfig::key_rate()
        )),
        &[InstrumentType::FIIndexTotalReturnSwap],
    );
}
