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
//! - Expected loss
//! - Jump to default

mod cs01;
mod expected_loss;
mod jump_to_default;
mod par_spread;
mod pv_premium;
mod pv_protection;
mod recovery01;
// risk_bucketed_dv01 - now using generic implementation
mod risky_pv01;

use crate::metrics::MetricRegistry;

/// Register all CDS Index metrics with the registry
pub(crate) fn register_cds_index_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{MetricCalculator, MetricId};
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    // Shared calculator for RiskyPv01 and custom "pv01" alias
    let risky_pv01_calc: Arc<dyn MetricCalculator> = Arc::new(risky_pv01::RiskyPv01Calculator);
    registry.register_metric(
        MetricId::RiskyPv01,
        Arc::clone(&risky_pv01_calc),
        &[InstrumentType::CDSIndex],
    );
    registry.register_metric(
        MetricId::custom("pv01"),
        risky_pv01_calc,
        &[InstrumentType::CDSIndex],
    );

    // Recovery01 (custom metric - recovery rate sensitivity)
    registry.register_metric(
        MetricId::Recovery01,
        Arc::new(recovery01::Recovery01Calculator),
        &[InstrumentType::CDSIndex],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::CDSIndex,
        metrics: [
            (ParSpread, par_spread::ParSpreadCalculator),
            (Cs01, cs01::Cs01Calculator),
            (Cs01Hazard, crate::metrics::GenericParallelCs01Hazard::<
                crate::instruments::CDSIndex,
            >::default()),
            (ProtectionLegPv, pv_protection::ProtectionLegPvCalculator),
            (PremiumLegPv, pv_premium::PremiumLegPvCalculator),
            (ExpectedLoss, expected_loss::ExpectedLossCalculator),
            (JumpToDefault, jump_to_default::JumpToDefaultCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CDSIndex,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            // Theta is now registered universally in metrics::standard_registry()
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CDSIndex,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }
}
