//! Pricer registrations for inflation instruments.
//!
//! Covers: InflationSwap, YoYInflationSwap, InflationCapFloor.

use super::*;

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register pricers for inflation instruments (swaps, caps/floors).
pub fn register_inflation_pricers(registry: &mut PricerRegistry) {
    // Inflation Swap
    register_pricer!(
        registry,
        InflationSwap,
        Discounting,
        crate::instruments::rates::inflation_swap::pricer::SimpleInflationSwapDiscountingPricer::default()
    );

    // YoY Inflation Swap
    register_pricer!(
        registry,
        YoYInflationSwap,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::rates::inflation_swap::YoYInflationSwap,
        >::discounting(InstrumentType::YoYInflationSwap)
    );

    // Inflation Cap/Floor
    register_pricer!(
        registry,
        InflationCapFloor,
        Black76,
        crate::instruments::rates::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::default()
    );
    register_pricer!(
        registry,
        InflationCapFloor,
        Normal,
        crate::instruments::rates::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::with_model(
            ModelKey::Normal
        )
    );
}
