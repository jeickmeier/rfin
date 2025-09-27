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

mod forward_points;
mod fx01;
mod ir01_domestic;
mod ir01_foreign;
// risk_bucketed_dv01 - now using generic implementation

use crate::metrics::MetricRegistry;

/// Register all FX Swap metrics with the registry
pub fn register_fx_swap_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
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
        )
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(
                crate::instruments::common::GenericBucketedDv01WithContext::<
                    crate::instruments::FxSwap,
                >::default(),
            ),
            &["FxSwap"],
        );
}
