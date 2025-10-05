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
mod pv;
// risk_bucketed_dv01 - now using generic implementation

pub use dv01::IrFutureDv01Calculator;
pub use pv::IrFuturePvCalculator;
// BucketedDv01Calculator now using generic implementation

use crate::metrics::MetricRegistry;

/// Register IR Future metrics with the registry
pub fn register_ir_future_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;
    
    // Custom metric
    registry.register_metric(
        MetricId::custom("ir_future_pv"),
        Arc::new(IrFuturePvCalculator),
        &["InterestRateFuture"],
    );
    
    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "InterestRateFuture",
        metrics: [
            (Dv01, IrFutureDv01Calculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::InterestRateFuture,
            >::default()),
        ]
    }
}
