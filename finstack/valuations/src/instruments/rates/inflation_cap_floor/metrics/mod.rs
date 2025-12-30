//! Inflation cap/floor metrics module.
//!
//! Provides metric calculators specific to `InflationCapFloor`, split into focused files.
//! Metrics are registered via `register_inflation_cap_floor_metrics`.

mod inflation01;
mod vega;

use crate::metrics::MetricRegistry;

/// Register all inflation cap/floor metrics with the registry.
pub fn register_inflation_cap_floor_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    registry.register_metric(
        MetricId::Inflation01,
        Arc::new(inflation01::Inflation01Calculator),
        &[InstrumentType::InflationCapFloor],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::InflationCapFloor,
        metrics: [
            (Vega, vega::VegaCalculator),
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InflationCapFloor,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::InflationCapFloor,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::InflationCapFloor,
            >::default()),
        ]
    };
}
