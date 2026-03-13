//! Pricer registrations for inflation instruments.
//!
//! Covers: InflationSwap, YoYInflationSwap, InflationCapFloor.

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for inflation instruments (swaps, caps/floors).
pub fn register_inflation_pricers(registry: &mut PricerRegistry) {
    // Inflation Swap
    registry.register(
        InstrumentType::InflationSwap,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::InflationSwap,
        >::discounting(InstrumentType::InflationSwap),
    );

    // YoY Inflation Swap
    registry.register(
        InstrumentType::YoYInflationSwap,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::rates::inflation_swap::YoYInflationSwap,
        >::discounting(InstrumentType::YoYInflationSwap),
    );

    // Inflation Cap/Floor
    registry.register(
        InstrumentType::InflationCapFloor,
        ModelKey::Black76,
        crate::instruments::rates::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::default(),
    );
    registry.register(
        InstrumentType::InflationCapFloor,
        ModelKey::Normal,
        crate::instruments::rates::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::with_model(
            ModelKey::Normal,
        ),
    );
}
