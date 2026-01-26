//! Commodity option metrics module.
//!
//! Registers option greeks and rate sensitivities for commodity options.
//!
//! # Commodity-Specific Greeks
//!
//! Unlike equity options, commodity options may not have a spot price scalar.
//! The gamma, vanna, and volga calculators are commodity-specific and handle
//! both scenarios:
//!
//! - **With `spot_price_id`**: bumps the spot price scalar
//! - **Without `spot_price_id`**: bumps the `PriceCurve` (parallel percent bump)
//!
//! This ensures FD greeks work correctly regardless of whether the option is
//! priced off spot or forward prices.

mod delta;
mod gamma;
mod vanna;
mod vega;
mod volga;

use crate::metrics::{MetricId, MetricRegistry};
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
    // Use commodity-specific gamma that handles PriceCurve bumping
    registry.register_metric(
        MetricId::Gamma,
        Arc::new(gamma::GammaCalculator),
        &[InstrumentType::CommodityOption],
    );
    // Use commodity-specific vanna that handles PriceCurve bumping
    registry.register_metric(
        MetricId::Vanna,
        Arc::new(vanna::VannaCalculator),
        &[InstrumentType::CommodityOption],
    );
    // Use commodity-specific volga (vol-only, same for spot or forward-based)
    registry.register_metric(
        MetricId::Volga,
        Arc::new(volga::VolgaCalculator),
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
            crate::metrics::Dv01CalculatorConfig::triangular_key_rate(),
        )),
        &[InstrumentType::CommodityOption],
    );
}
