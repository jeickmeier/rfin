//! Pricer registrations for commodity instruments.
//!
//! Covers: CommodityForward, CommoditySwap, CommodityOption,
//! CommodityAsianOption, CommoditySwaption, CommoditySpreadOption.

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for commodity instruments.
pub fn register_commodity_pricers(registry: &mut PricerRegistry) {
    // Commodity Forward
    registry.register(
        InstrumentType::CommodityForward,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CommodityForward,
        >::discounting(InstrumentType::CommodityForward),
    );

    // Commodity Swap
    registry.register(
        InstrumentType::CommoditySwap,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CommoditySwap,
        >::discounting(InstrumentType::CommoditySwap),
    );

    // Commodity Option
    registry.register(
        InstrumentType::CommodityOption,
        ModelKey::Black76,
        crate::instruments::commodity::commodity_option::pricer::CommodityOptionBlackPricer::default(),
    );
    registry.register(
        InstrumentType::CommodityOption,
        ModelKey::Discounting,
        crate::instruments::commodity::commodity_option::pricer::CommodityOptionBlackPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // Commodity Asian Option
    registry.register(
        InstrumentType::CommodityAsianOption,
        ModelKey::AsianTurnbullWakeman,
        crate::instruments::commodity::commodity_asian_option::pricer::CommodityAsianOptionAnalyticalPricer,
    );

    // Commodity Swaption
    registry.register(
        InstrumentType::CommoditySwaption,
        ModelKey::Black76,
        crate::instruments::commodity::commodity_swaption::pricer::CommoditySwaptionBlackPricer::default(),
    );

    // Commodity Spread Option (Kirk's approximation)
    registry.register(
        InstrumentType::CommoditySpreadOption,
        ModelKey::Black76,
        crate::instruments::commodity::commodity_spread_option::pricer::CommoditySpreadOptionKirkPricer::default(),
    );
}
