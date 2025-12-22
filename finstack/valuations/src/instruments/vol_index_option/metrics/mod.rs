//! Volatility index option metrics module.
//!
//! Provides metric calculators specific to `VolatilityIndexOption`, following
//! the shared metrics framework pattern.
//!
//! Exposed metrics:
//! - Delta (sensitivity to underlying volatility index level)
//! - Vega (sensitivity to volatility of volatility)
//! - DV01 (interest rate sensitivity)
//! - Theta (time decay)

mod delta;
mod vega;

use crate::metrics::MetricRegistry;

/// Register all VolatilityIndexOption metrics with the registry.
pub fn register_vol_index_option_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::VolatilityIndexOption,
        metrics: [
            (Delta, delta::DeltaCalculator),
            (Vega, vega::VegaCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::vol_index_option::VolatilityIndexOption,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::vol_index_option::VolatilityIndexOption,
            >::default()),
        ]
    }
}
