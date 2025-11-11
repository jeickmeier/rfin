//! FRA-specific metrics module.
//!
//! Provides metric calculators for FRAs, split into focused files for clarity
//! and parity with other instruments. Metrics include:
//! - PV passthrough (base value)
//! - DV01 (parallel rate sensitivity via generic calculator)
//!
//! See unit tests and `examples/` for usage.

mod par_rate;

use crate::metrics::{MetricId, MetricRegistry};
pub use par_rate::FraParRateCalculator;
use std::sync::Arc;

/// Registers all FRA metrics to a registry.
///
/// Each metric is registered with the "FRA" instrument type to ensure
/// proper applicability filtering.
pub fn register_fra_metrics(registry: &mut MetricRegistry) {
    // Custom metrics using GenericPv
    registry.register_metric(
        MetricId::custom("fra_pv"),
        Arc::new(crate::metrics::GenericPv),
        &["FRA"],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "FRA",
        metrics: [
            (Dv01, crate::metrics::GenericParallelDv01::<
                crate::instruments::ForwardRateAgreement,
            >::default()),
            (ParRate, FraParRateCalculator),
            (BucketedDv01, crate::metrics::GenericBucketedDv01WithContext::<
                crate::instruments::ForwardRateAgreement,
            >::default()),
        ]
    }
}
