//! Equity index future metrics module.
//!
//! Provides metric calculators specific to `EquityIndexFuture`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_equity_index_future_metrics`.
//!
//! Exposed metrics:
//! - DV01 (interest rate sensitivity)
//! - Bucketed DV01 (key rate sensitivity)
//! - Theta (time decay)

use crate::metrics::MetricRegistry;

/// Register all EquityIndexFuture metrics with the registry.
pub fn register_equity_index_future_metrics(registry: &mut MetricRegistry) {
    use crate::pricer::InstrumentType;
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::EquityIndexFuture,
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::equity_index_future::EquityIndexFuture,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::equity_index_future::EquityIndexFuture,
            >::new(crate::metrics::Dv01CalculatorConfig::key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::equity_index_future::EquityIndexFuture,
            >::default()),
        ]
    }
}

