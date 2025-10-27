//! IR Future metrics module.
//!
//! Provides metric calculators specific to `InterestRateFuture`, split into
//! focused files. The calculators compose with the shared metrics framework
//! and are registered via `register_ir_future_metrics`.
//!
//! Exposed metrics:
//! - PV passthrough (currency units)
//! - DV01 (per 1bp change in rate)

mod dv01;
// risk_bucketed_dv01 and theta now using generic implementations

pub use dv01::IrFutureDv01Calculator;
// PV and BucketedDv01 now using generic implementations

use crate::metrics::MetricRegistry;

/// Register IR Future metrics with the registry
pub fn register_ir_future_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metric using GenericPv
    registry.register_metric(
        MetricId::custom("ir_future_pv"),
        Arc::new(crate::instruments::common::metrics::GenericPv),
        &["InterestRateFuture"],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "InterestRateFuture",
        metrics: [
            (Dv01, IrFutureDv01Calculator),
            (Theta, crate::instruments::common::metrics::GenericTheta::<
                crate::instruments::InterestRateFuture,
            >::default()),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::InterestRateFuture,
            >::default()),
        ]
    }
}
