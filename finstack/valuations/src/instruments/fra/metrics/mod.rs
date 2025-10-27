//! FRA-specific metrics module.
//!
//! Provides metric calculators for FRAs, split into focused files for clarity
//! and parity with other instruments. Metrics include:
//! - PV passthrough (base value)
//! - Analytic DV01 approximation
//!
//! See unit tests and `examples/` for usage.

mod dv01;
mod par_rate;

use crate::metrics::{MetricId, MetricRegistry};
pub use dv01::FraDv01Calculator;
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
        Arc::new(crate::instruments::common::metrics::GenericPv),
        &["FRA"],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "FRA",
        metrics: [
            (Dv01, FraDv01Calculator),
            (ParRate, FraParRateCalculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::ForwardRateAgreement,
            >::default()),
        ]
    }
}
