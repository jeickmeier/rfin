//! CDS metrics module.
//!
//! Provides metric calculators specific to `CreditDefaultSwap`, split into
//! focused files. The calculators compose with the shared metrics framework
//! and are registered via `register_cds_metrics`.
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

/// Register all CDS metrics with the registry
pub fn register_cds_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{MetricCalculator, MetricId};
    use std::sync::Arc;

    // Shared calculator for RiskyPv01 and custom "pv01" alias
    let risky_pv01_calc: Arc<dyn MetricCalculator> = Arc::new(risky_pv01::RiskyPv01Calculator);
    registry.register_metric(MetricId::RiskyPv01, Arc::clone(&risky_pv01_calc), &["CDS"]);
    registry.register_metric(MetricId::custom("pv01"), risky_pv01_calc, &["CDS"]);

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "CDS",
        metrics: [
            (ParSpread, par_spread::ParSpreadCalculator),
            (Cs01, cs01::Cs01Calculator),
            (ProtectionLegPv, pv_protection::ProtectionLegPvCalculator),
            (PremiumLegPv, pv_premium::PremiumLegPvCalculator),
            (HazardCs01, hazard_cs01::HazardCs01Calculator),
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01WithContext::<
                crate::instruments::CreditDefaultSwap,
            >::default()),
        ]
    }
}
