//! Pricer registrations for FX instruments.
//!
//! Covers: FxSpot, FxSwap, XccySwap, FxOption, FxVarianceSwap, FxForward, Ndf,
//! FxBarrierOption, FxDigitalOption, FxTouchOption.

use super::*;

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register pricers for FX instruments.
pub fn register_fx_pricers(registry: &mut PricerRegistry) {
    // FX Spot
    register_pricer!(
        registry,
        FxSpot,
        Discounting,
        crate::instruments::fx::fx_spot::pricer::FxSpotPricer
    );

    // FX Swap
    register_pricer!(
        registry,
        FxSwap,
        Discounting,
        crate::instruments::fx::fx_swap::pricer::SimpleFxSwapDiscountingPricer::default()
    );

    // XCCY Swap
    register_pricer!(
        registry,
        XccySwap,
        Discounting,
        crate::instruments::rates::xccy_swap::pricer::SimpleXccySwapDiscountingPricer::default()
    );

    // FX Option
    register_pricer!(
        registry,
        FxOption,
        Black76,
        crate::instruments::fx::fx_option::pricer::SimpleFxOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        FxOption,
        Discounting,
        crate::instruments::fx::fx_option::pricer::SimpleFxOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // FX Variance Swap
    register_pricer!(
        registry,
        FxVarianceSwap,
        Discounting,
        crate::instruments::fx::fx_variance_swap::pricer::SimpleFxVarianceSwapDiscountingPricer::default()
    );

    // FX Forward - uses GenericInstrumentPricer (curve dependencies)
    register_pricer!(
        registry,
        FxForward,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::FxForward,
        >::discounting(InstrumentType::FxForward)
    );

    // NDF (Non-Deliverable Forward) - uses GenericInstrumentPricer (curve dependencies)
    register_pricer!(
        registry,
        Ndf,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<crate::instruments::Ndf>::discounting(
            InstrumentType::Ndf
        )
    );

    // FX Barrier Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        FxBarrierOption,
        MonteCarloGBM,
        crate::instruments::fx::fx_barrier_option::pricer::FxBarrierOptionMcPricer::default()
    );
    register_pricer!(
        registry,
        FxBarrierOption,
        FxBarrierBSContinuous,
        crate::instruments::fx::fx_barrier_option::pricer::FxBarrierOptionAnalyticalPricer
    );

    // FX Digital Option
    register_pricer!(
        registry,
        FxDigitalOption,
        Black76,
        crate::instruments::fx::fx_digital_option::SimpleFxDigitalOptionPricer::default()
    );
    register_pricer!(
        registry,
        FxDigitalOption,
        Discounting,
        crate::instruments::fx::fx_digital_option::SimpleFxDigitalOptionPricer::with_model(
            ModelKey::Discounting
        )
    );

    // FX Touch Option
    register_pricer!(
        registry,
        FxTouchOption,
        Black76,
        crate::instruments::fx::fx_touch_option::SimpleFxTouchOptionPricer::default()
    );
    register_pricer!(
        registry,
        FxTouchOption,
        Discounting,
        crate::instruments::fx::fx_touch_option::SimpleFxTouchOptionPricer::with_model(
            ModelKey::Discounting
        )
    );
}
