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

mod par_spread;
mod risky_pv01;
mod cs01;
mod pv_protection;
mod pv_premium;
mod hazard_cs01;

use crate::metrics::MetricRegistry;

/// Register all CDS Index metrics with the registry
pub fn register_cds_index_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry.register_metric(MetricId::ParSpread, Arc::new(par_spread::ParSpreadCalculator), &["CDSIndex"]);
    registry.register_metric(MetricId::RiskyPv01, Arc::new(risky_pv01::RiskyPv01Calculator), &["CDSIndex"]);
    registry.register_metric(MetricId::Cs01, Arc::new(cs01::Cs01Calculator), &["CDSIndex"]);
    registry.register_metric(MetricId::ProtectionLegPv, Arc::new(pv_protection::ProtectionLegPvCalculator), &["CDSIndex"]);
    registry.register_metric(MetricId::PremiumLegPv, Arc::new(pv_premium::PremiumLegPvCalculator), &["CDSIndex"]);
    registry.register_metric(MetricId::HazardCs01, Arc::new(hazard_cs01::HazardCs01Calculator), &["CDSIndex"]);
}


