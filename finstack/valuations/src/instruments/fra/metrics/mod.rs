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
mod pv;

use crate::metrics::{MetricCalculator, MetricId, MetricRegistry};
pub use dv01::FraDv01Calculator;
pub use par_rate::FraParRateCalculator;
pub use pv::FraPvCalculator;
use std::sync::Arc;

/// Registers all FRA metrics to a registry.
///
/// Each metric is registered with the "FRA" instrument type to ensure
/// proper applicability filtering.
pub fn register_fra_metrics(registry: &mut MetricRegistry) {
    let dv01_calc: Arc<dyn MetricCalculator> = Arc::new(FraDv01Calculator);

    registry
        .register_metric(
            MetricId::custom("fra_pv"),
            Arc::new(FraPvCalculator),
            &["FRA"],
        ) // PV passthrough
        .register_metric(MetricId::Dv01, Arc::clone(&dv01_calc), &["FRA"]) // Standard DV01 id
        .register_metric(MetricId::custom("pv01"), dv01_calc, &["FRA"]) // Alias for DV01
        .register_metric(MetricId::ParRate, Arc::new(FraParRateCalculator), &["FRA"]) // Par rate
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(
                crate::instruments::common::GenericBucketedDv01WithContext::<
                    crate::instruments::ForwardRateAgreement,
                >::default(),
            ),
            &["FRA"],
        );
}
