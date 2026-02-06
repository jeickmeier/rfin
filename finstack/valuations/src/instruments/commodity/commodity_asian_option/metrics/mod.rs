//! Commodity Asian option metrics module.
//!
//! Provides rate sensitivities (DV01) for commodity Asian options.
//! Greeks (delta, vega) are not registered here because the commodity Asian
//! option uses forward curve pricing, not spot-based pricing.

use crate::metrics::{MetricId, MetricRegistry};
use crate::pricer::InstrumentType;
use std::sync::Arc;

/// Register commodity Asian option metrics with the registry.
pub fn register_commodity_asian_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Dv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::commodity::commodity_asian_option::CommodityAsianOption,
        >::new(
            crate::metrics::Dv01CalculatorConfig::parallel_combined()
        )),
        &[InstrumentType::CommodityAsianOption],
    );
    registry.register_metric(
        MetricId::BucketedDv01,
        Arc::new(crate::metrics::UnifiedDv01Calculator::<
            crate::instruments::commodity::commodity_asian_option::CommodityAsianOption,
        >::new(
            crate::metrics::Dv01CalculatorConfig::triangular_key_rate(),
        )),
        &[InstrumentType::CommodityAsianOption],
    );
}
