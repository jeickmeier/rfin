//! FX Swap metrics module.
//!
//! Provides metric calculators specific to `FxSwap`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_fx_swap_metrics`.
//!
//! Exposed metrics:
//! - Forward points (far rate - near rate)
//! - FX01 (sensitivity to 1bp spot bump)
//! - DV01 (domestic) and DV01 (foreign)

mod carry_pv;
mod forward_points;
mod fx01;
mod fx_delta;
mod ir01_domestic;
mod ir01_foreign;
// dv01, risk_bucketed_dv01 and theta now using generic implementations

use crate::metrics::MetricRegistry;

/// Register all FX Swap metrics with the registry
pub(crate) fn register_fx_swap_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    // Custom metrics
    registry
        .register_metric(
            MetricId::custom("carry_pv"),
            Arc::new(carry_pv::CarryPv),
            &[InstrumentType::FxSwap],
        )
        .register_metric(
            MetricId::custom("forward_points"),
            Arc::new(forward_points::ForwardPoints),
            &[InstrumentType::FxSwap],
        )
        .register_metric(
            MetricId::Fx01,
            Arc::new(fx01::FX01),
            &[InstrumentType::FxSwap],
        )
        .register_metric(
            MetricId::FxDelta,
            Arc::new(fx_delta::FxDeltaCalculator),
            &[InstrumentType::FxSwap],
        )
        .register_metric(
            MetricId::Dv01Domestic,
            Arc::new(ir01_domestic::DomesticIR01),
            &[InstrumentType::FxSwap],
        )
        .register_metric(
            MetricId::Dv01Foreign,
            Arc::new(ir01_foreign::ForeignIR01),
            &[InstrumentType::FxSwap],
        );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxSwap,
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }
}
