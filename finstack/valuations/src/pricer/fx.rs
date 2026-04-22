//! Pricer registrations for FX instruments.
//!
//! Covers: FxSpot, FxSwap, XccySwap, FxOption, FxVarianceSwap, FxForward, Ndf,
//! FxBarrierOption, FxDigitalOption, FxTouchOption.

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for FX instruments.
pub(crate) fn register_fx_pricers(registry: &mut PricerRegistry) {
    // FX Spot
    registry.register(
        InstrumentType::FxSpot,
        ModelKey::Discounting,
        crate::instruments::fx::fx_spot::pricer::FxSpotPricer,
    );

    // FX Swap
    registry.register(
        InstrumentType::FxSwap,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::FxSwap,
        >::discounting(InstrumentType::FxSwap),
    );

    // XCCY Swap
    registry.register(
        InstrumentType::XccySwap,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::XccySwap,
        >::discounting(InstrumentType::XccySwap),
    );

    // FX Option
    registry.register(
        InstrumentType::FxOption,
        ModelKey::Black76,
        crate::instruments::fx::fx_option::pricer::SimpleFxOptionBlackPricer::default(),
    );
    registry.register(
        InstrumentType::FxOption,
        ModelKey::Discounting,
        crate::instruments::fx::fx_option::pricer::SimpleFxOptionBlackPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // FX Variance Swap
    registry.register(
        InstrumentType::FxVarianceSwap,
        ModelKey::Discounting,
        crate::instruments::fx::fx_variance_swap::pricer::SimpleFxVarianceSwapDiscountingPricer,
    );

    // FX Forward - uses GenericInstrumentPricer (curve dependencies)
    registry.register(
        InstrumentType::FxForward,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::FxForward,
        >::discounting(InstrumentType::FxForward),
    );

    // NDF (Non-Deliverable Forward) - uses GenericInstrumentPricer (curve dependencies)
    registry.register(
        InstrumentType::Ndf,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<crate::instruments::Ndf>::discounting(
            InstrumentType::Ndf,
        ),
    );

    // FX Barrier Option
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::FxBarrierOption,
        ModelKey::MonteCarloGBM,
        crate::instruments::fx::fx_barrier_option::pricer::FxBarrierOptionMcPricer::default(),
    );
    registry.register(
        InstrumentType::FxBarrierOption,
        ModelKey::FxBarrierBSContinuous,
        crate::instruments::fx::fx_barrier_option::pricer::FxBarrierOptionAnalyticalPricer,
    );
    // Audit P2 #29: Vanna-Volga smile-corrected FX barrier. Defaults to a
    // degenerate symmetric smile (equivalent to BS) until callers bind
    // real market quotes via `FxBarrierOptionVannaVolgaPricer::with_quotes`.
    registry.register(
        InstrumentType::FxBarrierOption,
        ModelKey::FxBarrierVannaVolga,
        crate::instruments::fx::fx_barrier_option::pricer::FxBarrierOptionVannaVolgaPricer::new(),
    );

    // FX Digital Option
    registry.register(
        InstrumentType::FxDigitalOption,
        ModelKey::Black76,
        crate::instruments::fx::fx_digital_option::SimpleFxDigitalOptionPricer::default(),
    );
    registry.register(
        InstrumentType::FxDigitalOption,
        ModelKey::Discounting,
        crate::instruments::fx::fx_digital_option::SimpleFxDigitalOptionPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // FX Touch Option
    registry.register(
        InstrumentType::FxTouchOption,
        ModelKey::Black76,
        crate::instruments::fx::fx_touch_option::SimpleFxTouchOptionPricer::default(),
    );
    registry.register(
        InstrumentType::FxTouchOption,
        ModelKey::Discounting,
        crate::instruments::fx::fx_touch_option::SimpleFxTouchOptionPricer::with_model(
            ModelKey::Discounting,
        ),
    );
}
