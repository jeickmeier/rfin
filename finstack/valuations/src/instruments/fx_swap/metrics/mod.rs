//! FX Swap metrics module.
//!
//! Provides metric calculators specific to `FxSwap`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_fx_swap_metrics`.
//!
//! Exposed metrics:
//! - Forward points (far rate - near rate)
//! - FX01 (sensitivity to 1bp spot bump)
//! - IR01 (domestic) and IR01 (foreign)

mod carry_pv;
mod dv01;
mod forward_points;
mod fx01;
mod ir01_domestic;
mod ir01_foreign;
mod theta;
// risk_bucketed_dv01 - now using generic implementation

use crate::metrics::MetricRegistry;

/// Register all FX Swap metrics with the registry
pub fn register_fx_swap_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metrics
    registry
        .register_metric(
            MetricId::custom("carry_pv"),
            Arc::new(carry_pv::CarryPv),
            &["FxSwap"],
        )
        .register_metric(
            MetricId::custom("forward_points"),
            Arc::new(forward_points::ForwardPoints),
            &["FxSwap"],
        )
        .register_metric(MetricId::custom("fx01"), Arc::new(fx01::FX01), &["FxSwap"])
        .register_metric(
            MetricId::custom("ir01_domestic"),
            Arc::new(ir01_domestic::DomesticIR01),
            &["FxSwap"],
        )
        .register_metric(
            MetricId::custom("ir01_foreign"),
            Arc::new(ir01_foreign::ForeignIR01),
            &["FxSwap"],
        );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "FxSwap",
        metrics: [
            (Dv01, dv01::FxSwapDv01Calculator),
            (Theta, theta::ThetaCalculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::FxSwap,
            >::default()),
        ]
    }
}
