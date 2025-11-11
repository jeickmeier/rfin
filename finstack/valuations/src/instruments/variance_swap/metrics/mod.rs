//! Variance swap metrics module.
//!
//! Split into focused calculators similar to other instruments. This `mod.rs`
//! re-exports the calculators and provides a registry hookup.

pub mod dv01;
pub mod expected_variance;
pub mod notional;
pub mod realized_variance;
pub mod strike_vol;
pub mod time_to_maturity;
pub mod variance_vega;
pub mod vega;

pub use dv01::Dv01Calculator;
pub use expected_variance::ExpectedVarianceCalculator;
pub use notional::VarianceNotionalCalculator;
pub use realized_variance::RealizedVarianceCalculator;
pub use strike_vol::StrikeVolCalculator;
pub use time_to_maturity::TimeToMaturityCalculator;
pub use variance_vega::VarianceVegaCalculator;
pub use vega::VegaCalculator;

use crate::metrics::MetricRegistry;

/// Register variance swap metrics with the registry.
pub fn register_variance_swap_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "VarianceSwap",
        metrics: [
            (Vega, VegaCalculator),
            (Dv01, Dv01Calculator),
            (VarianceVega, VarianceVegaCalculator),
            (ExpectedVariance, ExpectedVarianceCalculator),
            (RealizedVariance, RealizedVarianceCalculator),
            (VarianceNotional, VarianceNotionalCalculator),
            (VarianceStrikeVol, StrikeVolCalculator),
            (VarianceTimeToMaturity, TimeToMaturityCalculator),
        ]
    };
}
