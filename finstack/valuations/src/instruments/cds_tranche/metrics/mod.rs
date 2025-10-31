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
//! - Correlation01 (per 1% correlation change)

mod correlation01;
mod cs01;
mod dv01;
mod expected_loss;
mod jump_to_default;
mod par_spread;
mod recovery01;
// risk_bucketed_dv01 - now using generic implementation
mod spread_dv01;
mod theta;
mod upfront;

use crate::metrics::MetricRegistry;

/// Register all CDS Tranche metrics with the registry
pub fn register_cds_tranche_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    // Custom metrics (legacy names for backward compatibility)
    registry
        .register_metric(
            MetricId::custom("upfront"),
            Arc::new(upfront::UpfrontCalculator),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::SpreadDv01,
            Arc::new(spread_dv01::SpreadDv01Calculator),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::Correlation01,
            Arc::new(correlation01::Correlation01Calculator),
            &["CDSTranche"],
        )
        .register_metric(
            MetricId::Recovery01,
            Arc::new(recovery01::Recovery01Calculator),
            &["CDSTranche"],
        );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "CDSTranche",
        metrics: [
            (Cs01, cs01::Cs01Calculator),
            (ParSpread, par_spread::ParSpreadCalculator),
            (ExpectedLoss, expected_loss::ExpectedLossCalculator),
            (JumpToDefault, jump_to_default::JumpToDefaultCalculator),
            (Dv01, dv01::CdsTrancheDv01Calculator),
            (Theta, theta::ThetaCalculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::CdsTranche,
            >::default()),
        ]
    }
}
