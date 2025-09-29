//! CDS Index metrics module.
//!
//! Provides metric calculators specific to `CDSIndex`, split into focused
//! files. The calculators compose with the shared metrics framework and are
//! registered via `register_cds_index_metrics`.
//!
//! Exposed metrics:
//! - Par spread (bps)
//! - Risky PV01
//! - CS01 (approximate)
//! - Protection leg PV
//! - Premium leg PV

mod cs01;
mod hazard_cs01;
mod par_spread;
mod pv_premium;
mod pv_protection;
// risk_bucketed_dv01 - now using generic implementation
mod risky_pv01;

use crate::metrics::MetricRegistry;

/// Register all CDS Index metrics with the registry
pub fn register_cds_index_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{MetricCalculator, MetricId};
    use std::sync::Arc;

    registry.register_metric(
        MetricId::ParSpread,
        Arc::new(par_spread::ParSpreadCalculator),
        &["CDSIndex"],
    );

    let risky_pv01_calc: Arc<dyn MetricCalculator> = Arc::new(risky_pv01::RiskyPv01Calculator);
    registry.register_metric(
        MetricId::RiskyPv01,
        Arc::clone(&risky_pv01_calc),
        &["CDSIndex"],
    );
    registry.register_metric(MetricId::custom("pv01"), risky_pv01_calc, &["CDSIndex"]);
    registry.register_metric(
        MetricId::Cs01,
        Arc::new(cs01::Cs01Calculator),
        &["CDSIndex"],
    );
    registry.register_metric(
        MetricId::ProtectionLegPv,
        Arc::new(pv_protection::ProtectionLegPvCalculator),
        &["CDSIndex"],
    );
    registry.register_metric(
        MetricId::PremiumLegPv,
        Arc::new(pv_premium::PremiumLegPvCalculator),
        &["CDSIndex"],
    );
    registry.register_metric(
        MetricId::HazardCs01,
        Arc::new(hazard_cs01::HazardCs01Calculator),
        &["CDSIndex"],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(
            crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::CDSIndex,
            >::default(),
        ),
        &["CDSIndex"],
    );
}
