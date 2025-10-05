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
mod par_spread;
// risk_bucketed_dv01 - now using generic implementation
mod spread_dv01;
mod upfront;

use crate::metrics::MetricRegistry;

/// Register all CDS Tranche metrics with the registry
pub fn register_cds_tranche_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metrics
    registry
        .register_metric(
            MetricId::custom("upfront"),
            Arc::new(upfront::UpfrontCalculator),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::custom("spread_dv01"),
            Arc::new(spread_dv01::SpreadDv01Calculator),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::custom("cs01"),
            Arc::new(cs01::Cs01Calculator),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::custom("correlation_delta"),
            Arc::new(correlation_delta::CorrelationDeltaCalculator),
            &["CDSTranche"],
        );
    
    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "CDSTranche",
        metrics: [
            (ParSpread, par_spread::ParSpreadCalculator),
            (ExpectedLoss, expected_loss::ExpectedLossCalculator),
            (JumpToDefault, jump_to_default::JumpToDefaultCalculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::CdsTranche,
            >::default()),
        ]
    }
}
