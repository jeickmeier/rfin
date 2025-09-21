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

pub use dv01::IrFutureDv01Calculator;
pub use pv::IrFuturePvCalculator;

use crate::metrics::{MetricId, MetricRegistry};
use std::sync::Arc;

/// Register IR Future metrics with the registry
pub fn register_ir_future_metrics(registry: &mut MetricRegistry) {
    registry
        .register_metric(
            MetricId::custom("ir_future_pv"),
            Arc::new(IrFuturePvCalculator),
            &["InterestRateFuture"],
        )
        .register_metric(
            MetricId::Dv01,
            Arc::new(IrFutureDv01Calculator),
            &["InterestRateFuture"],
        );
}


