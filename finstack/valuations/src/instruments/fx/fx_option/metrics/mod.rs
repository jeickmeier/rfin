//! FX option metrics module.
//!
//! Splits FX option metrics into focused calculators per greek and registers
//! them with the `MetricRegistry`. Calculators reuse the pricing engine
//! helpers to ensure consistency between PV and greeks.

mod implied_vol;
// delta/gamma/vega/theta/rho/vanna/volga are provided via common adapters
// dv01 and bucketed_dv01 now using generic implementations

use crate::metrics::MetricRegistry;

/// Register FX option metrics with the registry.
pub fn register_fx_option_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    // Standard metrics for rho split by domestic/foreign.
    registry.register_metric(
        MetricId::Rho,
        Arc::new(crate::metrics::OptionRhoCalculator::<
            crate::instruments::FxOption,
        >::default()),
        &[InstrumentType::FxOption],
    );
    registry.register_metric(
        MetricId::ForeignRho,
        Arc::new(crate::metrics::OptionForeignRhoCalculator::<
            crate::instruments::FxOption,
        >::default()),
        &[InstrumentType::FxOption],
    );

    // Backwards-compatible aliases (kept for stability).
    registry.register_metric(
        MetricId::custom("rho_domestic"),
        Arc::new(crate::metrics::OptionRhoCalculator::<
            crate::instruments::FxOption,
        >::default()),
        &[InstrumentType::FxOption],
    );
    registry.register_metric(
        MetricId::custom("rho_foreign"),
        Arc::new(crate::metrics::OptionForeignRhoCalculator::<
            crate::instruments::FxOption,
        >::default()),
        &[InstrumentType::FxOption],
    );

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxOption,
        metrics: [
            (Delta, crate::metrics::OptionDeltaCalculator::<crate::instruments::FxOption>::default()),
            (Gamma, crate::metrics::OptionGammaCalculator::<crate::instruments::FxOption>::default()),
            (Vega, crate::metrics::OptionVegaCalculator::<crate::instruments::FxOption>::default()),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            // Override universal theta (carry) with model theta for FX options.
            (Theta, crate::metrics::OptionThetaCalculator::<crate::instruments::FxOption>::default()),
            (ImpliedVol, implied_vol::ImpliedVolCalculator),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxOption,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Vanna, crate::metrics::OptionVannaCalculator::<crate::instruments::FxOption>::default()),
            (Volga, crate::metrics::OptionVolgaCalculator::<crate::instruments::FxOption>::default()),
        ]
    }
}
