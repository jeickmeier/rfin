//! FX option metrics module.
//!
//! Splits FX option metrics into focused calculators per greek and registers
//! them with the `MetricRegistry`. Calculators reuse the pricing engine
//! helpers to ensure consistency between PV and greeks.

mod delta;
mod gamma;
mod implied_vol;
mod rho;
mod vanna;
mod vega;
mod volga;
// dv01 and bucketed_dv01 now using generic implementations

use crate::metrics::MetricRegistry;

/// Register FX option metrics with the registry.
pub fn register_fx_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    // Custom metrics for rho split by domestic/foreign
    registry.register_metric(
        MetricId::custom("rho_domestic"),
        Arc::new(rho::RhoDomesticCalculator),
        &[InstrumentType::FxOption],
    );
    registry.register_metric(
        MetricId::custom("rho_foreign"),
        Arc::new(rho::RhoForeignCalculator),
        &[InstrumentType::FxOption],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxOption,
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Gamma, gamma::GammaCalculator),
            (Vega, vega::VegaCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            // Theta is now registered universally in metrics::standard_registry()
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Vanna, vanna::VannaCalculator),
            (Volga, volga::VolgaCalculator),
        ]
    }
}
