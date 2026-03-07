//! Pricer registrations for commodity instruments.
//!
//! Covers: CommodityForward, CommoditySwap, CommodityOption,
//! CommoditySwaption, CommoditySpreadOption.

use super::*;

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register pricers for commodity instruments.
pub fn register_commodity_pricers(registry: &mut PricerRegistry) {
    // Commodity Forward
    register_pricer!(
        registry,
        CommodityForward,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CommodityForward,
        >::discounting(InstrumentType::CommodityForward)
    );

    // Commodity Swap
    register_pricer!(
        registry,
        CommoditySwap,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::CommoditySwap,
        >::discounting(InstrumentType::CommoditySwap)
    );

    // Commodity Option
    register_pricer!(
        registry,
        CommodityOption,
        Black76,
        crate::instruments::commodity::commodity_option::pricer::CommodityOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        CommodityOption,
        Discounting,
        crate::instruments::commodity::commodity_option::pricer::CommodityOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Commodity Swaption
    register_pricer!(
        registry,
        CommoditySwaption,
        Black76,
        crate::instruments::commodity::commodity_swaption::pricer::CommoditySwaptionBlackPricer::default()
    );

    // Commodity Spread Option (Kirk's approximation)
    register_pricer!(
        registry,
        CommoditySpreadOption,
        Black76,
        crate::instruments::commodity::commodity_spread_option::pricer::CommoditySpreadOptionKirkPricer::default()
    );
}
