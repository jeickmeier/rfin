//! IR Future metrics module.
//!
//! Provides metric calculators specific to `InterestRateFuture`, split into
//! focused files. The calculators compose with the shared metrics framework
//! and are registered via `register_ir_future_metrics`.
//!
//! Exposed metrics:
//! - PV passthrough (currency units)
//! - DV01 (parallel rate sensitivity via generic calculator)

// All metrics now using generic implementations

use crate::metrics::MetricRegistry;

/// Register IR Future metrics with the registry
pub fn register_ir_future_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metric using GenericPv
    registry.register_metric(
        MetricId::custom("ir_future_pv"),
        Arc::new(crate::metrics::GenericPv),
        &["InterestRateFuture"],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "InterestRateFuture",
        metrics: [
            (Dv01, crate::metrics::GenericParallelDv01::<
                crate::instruments::InterestRateFuture,
            >::default()),
            // Theta is now registered universally in metrics::standard_registry()
            (BucketedDv01, crate::metrics::GenericBucketedDv01WithContext::<
                crate::instruments::InterestRateFuture,
            >::default()),
        ]
    }
}
