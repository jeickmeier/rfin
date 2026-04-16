//! Snowball / Inverse Floater metrics module.

#[cfg(feature = "mc")]
use crate::metrics::MetricRegistry;

/// Register snowball metrics with the registry.
#[cfg(feature = "mc")]
pub fn register_snowball_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::{Dv01CalculatorConfig, MetricId, UnifiedDv01Calculator};
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    registry.register_metric(
        MetricId::Dv01,
        Arc::new(UnifiedDv01Calculator::<super::Snowball>::new(
            Dv01CalculatorConfig::parallel_combined(),
        )),
        &[InstrumentType::Snowball],
    );

    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::Snowball,
        metrics: [
            (BucketedDv01, UnifiedDv01Calculator::<super::Snowball>::new(
                Dv01CalculatorConfig::triangular_key_rate(),
            )),
        ]
    }
}
