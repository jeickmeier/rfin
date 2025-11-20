//! CDS metrics module.
//!
//! Provides metric calculators specific to `CreditDefaultSwap`, split into
//! focused files. The calculators compose with the shared metrics framework
//! and are registered via `register_cds_metrics`.
//!
//! Exposed metrics:
//! - Par spread (bps)
//! - Risky PV01
//! - CS01
//! - Protection leg PV
//! - Premium leg PV
//! - Expected loss
//! - Jump to default

mod cs_gamma;
mod expected_loss;
mod jump_to_default;
mod par_spread;
mod pv_premium;
mod pv_protection;
mod recovery01;
// risk_bucketed_dv01 and theta now using generic implementations
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

    // Recovery01 (custom metric - recovery rate sensitivity)
    registry.register_metric(
        MetricId::Recovery01,
        Arc::new(recovery01::Recovery01Calculator),
        &["CDS"],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: "CDS",
        metrics: [
            (ParSpread, par_spread::ParSpreadCalculator),
            (Cs01, crate::metrics::GenericParallelCs01::<
                crate::instruments::CreditDefaultSwap,
            >::default()),
            (CsGamma, cs_gamma::CsGammaCalculator),
            (ProtectionLegPv, pv_protection::ProtectionLegPvCalculator),
            (PremiumLegPv, pv_premium::PremiumLegPvCalculator),
            (ExpectedLoss, expected_loss::ExpectedLossCalculator),
            (JumpToDefault, jump_to_default::JumpToDefaultCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CreditDefaultSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::CreditDefaultSwap,
            >::default()),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CreditDefaultSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
        ]
    }
}
