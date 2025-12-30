//! Dollar roll risk metrics.
//!
//! Dollar roll specific metrics include implied financing rate,
//! roll specialness, and break-even analysis.

// Metrics are implemented in the carry module
// Re-export key functions here

pub use super::carry::{break_even_drop, implied_financing_rate, roll_specialness, CarryResult};

use crate::metrics::MetricRegistry;

/// Register dollar roll metrics with the registry.
pub fn register_dollar_roll_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::DollarRoll,
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::DollarRoll,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::DollarRoll,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::DollarRoll,
            >::default()),
        ]
    }
}
