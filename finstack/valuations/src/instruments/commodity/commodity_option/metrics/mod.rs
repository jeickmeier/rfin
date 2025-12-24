//! Commodity option metrics module.
//!
//! Registers core option greeks using generic finite-difference calculators.

use crate::metrics::{GenericFdGamma, GenericFdVanna, GenericFdVolga, MetricId, MetricRegistry};
use crate::pricer::InstrumentType;
use std::sync::Arc;

/// Register commodity option metrics with the registry.
pub fn register_commodity_option_metrics(registry: &mut MetricRegistry) {
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
}
