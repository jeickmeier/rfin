//! FX variance swap metrics module.
//!
//! Split into focused calculators similar to equity variance swaps. This `mod.rs`
//! re-exports the calculators and provides a registry hook.

pub mod expected_variance;
pub mod notional;
pub mod realized_variance;
pub mod strike_vol;
pub mod time_to_maturity;
pub mod variance_vega;
pub mod vega;

pub use expected_variance::ExpectedVarianceCalculator;
pub use notional::VarianceNotionalCalculator;
pub use realized_variance::RealizedVarianceCalculator;
pub use strike_vol::StrikeVolCalculator;
pub use time_to_maturity::TimeToMaturityCalculator;
pub use variance_vega::VarianceVegaCalculator;
pub use vega::VegaCalculator;

use crate::metrics::MetricRegistry;

/// Register FX variance swap metrics with the registry.
pub fn register_fx_variance_swap_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxVarianceSwap,
        metrics: [
            (Vega, VegaCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxVarianceSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::FxVarianceSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
            (VarianceVega, VarianceVegaCalculator),
            (ExpectedVariance, ExpectedVarianceCalculator),
            (RealizedVariance, RealizedVarianceCalculator),
            (VarianceNotional, VarianceNotionalCalculator),
            (VarianceStrikeVol, StrikeVolCalculator),
            (VarianceTimeToMaturity, TimeToMaturityCalculator),
        ]
    };
}
