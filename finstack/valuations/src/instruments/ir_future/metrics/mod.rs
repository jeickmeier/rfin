//! IR Future metrics module.
//!
//! Provides metric calculators specific to `InterestRateFuture`, split into
//! focused files. The calculators compose with the shared metrics framework
//! and are registered via `register_ir_future_metrics`.
//!
//! Exposed metrics:
//! - DV01 (parallel rate sensitivity via generic calculator)
//! - Bucketed DV01 (key-rate sensitivity)
//!
//! Note: PV is available in `ValuationResult.value`, not as a metric in measures.

use crate::metrics::MetricRegistry;

/// Register IR Future metrics with the registry
pub fn register_ir_future_metrics(registry: &mut MetricRegistry) {
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
