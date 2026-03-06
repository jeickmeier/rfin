//! Volatility index future metrics module.
//!
//! Provides metric calculators specific to `VolatilityIndexFuture`, following
//! the shared metrics framework pattern.
//!
//! Exposed metrics:
//! - DeltaVol (exposure to underlying volatility index level)
//! - DV01 (interest rate sensitivity - minimal for vol futures)
//! - Theta (time decay)

mod delta_vol;

use crate::metrics::{MetricId, MetricRegistry};

/// Register all VolatilityIndexFuture metrics with the registry.
pub fn register_vol_index_future_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    // Register custom DeltaVol metric (not a standard MetricId)
    registry.register_metric(
        MetricId::custom("delta_vol"),
        Arc::new(delta_vol::DeltaVolCalculator),
        &[InstrumentType::VolatilityIndexFuture],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::VolatilityIndexFuture,
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::equity::vol_index_future::VolatilityIndexFuture,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
        ]
    }
}
