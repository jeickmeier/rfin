//! Commodity option metrics module.
//!
//! Registers option greeks and rate sensitivities for commodity options.

mod delta;
mod vega;

use crate::metrics::{GenericFdGamma, GenericFdVanna, GenericFdVolga, MetricId, MetricRegistry};
use crate::pricer::InstrumentType;
use std::sync::Arc;

/// Register commodity option metrics with the registry.
pub fn register_commodity_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(delta::DeltaCalculator),
        &[InstrumentType::CommodityOption],
    );
    registry.register_metric(
        MetricId::Vega,
        Arc::new(vega::VegaCalculator),
        &[InstrumentType::CommodityOption],
    );
    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<crate::instruments::CommodityOption>::default()),
        &[InstrumentType::CommodityOption],
    );
    registry.register_metric(
        MetricId::Vanna,
        Arc::new(GenericFdVanna::<crate::instruments::CommodityOption>::default()),
        &[InstrumentType::CommodityOption],
    );
    registry.register_metric(
        MetricId::Volga,
        Arc::new(GenericFdVolga::<crate::instruments::CommodityOption>::default()),
        &[InstrumentType::CommodityOption],
    );
    registry.register_metric(
        MetricId::Dv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::CommodityOption,
        >::new(
            crate::metrics::Dv01CalculatorConfig::parallel_combined()
        )),
        &[InstrumentType::CommodityOption],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::CommodityOption,
        >::new(
            crate::metrics::Dv01CalculatorConfig::triangular_key_rate()
        )),
        &[InstrumentType::CommodityOption],
    );
}
