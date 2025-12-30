//! FX forward metrics module.
//!
//! Provides metric calculators specific to `FxForward`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_fx_forward_metrics`.
//!
//! Exposed metrics:
//! - DV01 (interest rate sensitivity for domestic and foreign curves)
//! - Theta (time decay)

mod fx01;

use crate::metrics::MetricRegistry;

/// Register all FxForward metrics with the registry.
pub fn register_fx_forward_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    registry.register_metric(
        MetricId::Fx01,
        Arc::new(fx01::Fx01Calculator),
        &[InstrumentType::FxForward],
    );
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::FxForward,
        metrics: [
            (Dv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::fx_forward::FxForward,
            >::new(crate::metrics::Dv01CalculatorConfig::parallel_combined())),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::fx_forward::FxForward,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
            (Theta, crate::metrics::GenericTheta::<
                crate::instruments::fx_forward::FxForward,
            >::default()),
        ]
    }
}
