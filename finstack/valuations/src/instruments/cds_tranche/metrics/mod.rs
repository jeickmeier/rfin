//! CDS Tranche metrics module.
//!
//! Provides metric calculators specific to `CdsTranche`, split into focused
//! files. The calculators compose with the shared metrics framework and are
//! registered via `register_cds_tranche_metrics`.
//!
//! Exposed metrics:
//! - Upfront (currency units)
//! - Spread DV01 (per 1bp change in running coupon)
//! - Expected loss (currency units)
//! - Jump-to-default (currency units)
//! - CS01 (approximate parallel spread shift)
//! - Correlation delta (per unit correlation change)

mod correlation_delta;
mod cs01;
mod expected_loss;
mod jump_to_default;
mod spread_dv01;
mod upfront;

use crate::metrics::MetricRegistry;

/// Register all CDS Tranche metrics with the registry
pub fn register_cds_tranche_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::custom("upfront"),
            Arc::new(upfront::UpfrontCalculator),
            &["CDSTranche"],
        ) // upfront PV
        .register_metric(
            MetricId::custom("spread_dv01"),
            Arc::new(spread_dv01::SpreadDv01Calculator),
            &["CDSTranche"],
        ) // running coupon DV01
        .register_metric(
            MetricId::ExpectedLoss,
            Arc::new(expected_loss::ExpectedLossCalculator),
            &["CDSTranche"],
        ) // total expected loss
        .register_metric(
            MetricId::JumpToDefault,
            Arc::new(jump_to_default::JumpToDefaultCalculator),
            &["CDSTranche"],
        ) // JTD amount
        .register_metric(
            MetricId::custom("cs01"),
            Arc::new(cs01::Cs01Calculator),
            &["CDSTranche"],
        ) // index hazard bump CS01
        .register_metric(
            MetricId::custom("correlation_delta"),
            Arc::new(correlation_delta::CorrelationDeltaCalculator),
            &["CDSTranche"],
        );
}
