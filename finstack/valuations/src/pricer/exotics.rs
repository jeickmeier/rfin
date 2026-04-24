//! Pricer registrations for exotic instruments.
//!
//! Covers: Basket, AsianOption, BarrierOption, LookbackOption, QuantoOption,
//! Autocallable, CmsOption, CmsSwap, CliquetOption, RangeAccrual, BermudanSwaption.

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for exotic instruments (barriers, lookbacks, Asians,
/// autocallables, quantos, cliquets, range accruals, Bermudan swaptions).
pub(crate) fn register_exotic_pricers(registry: &mut PricerRegistry) {
    // Basket
    registry.register(
        InstrumentType::Basket,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::exotics::basket::Basket,
        >::discounting(InstrumentType::Basket),
    );

    // Asian Option

    registry.register(
        InstrumentType::AsianOption,
        ModelKey::MonteCarloGBM,
        crate::instruments::exotics::asian_option::pricer::AsianOptionMcPricer::default(),
    );
    registry.register(
        InstrumentType::AsianOption,
        ModelKey::AsianGeometricBS,
        crate::instruments::exotics::asian_option::pricer::AsianOptionAnalyticalGeometricPricer,
    );
    registry.register(
        InstrumentType::AsianOption,
        ModelKey::AsianTurnbullWakeman,
        crate::instruments::exotics::asian_option::pricer::AsianOptionSemiAnalyticalTwPricer,
    );

    // Barrier Option

    registry.register(
        InstrumentType::BarrierOption,
        ModelKey::MonteCarloGBM,
        crate::instruments::exotics::barrier_option::pricer::BarrierOptionMcPricer::default(),
    );
    registry.register(
        InstrumentType::BarrierOption,
        ModelKey::BarrierBSContinuous,
        crate::instruments::exotics::barrier_option::pricer::BarrierOptionAnalyticalPricer,
    );

    // Lookback Option

    registry.register(
        InstrumentType::LookbackOption,
        ModelKey::MonteCarloGBM,
        crate::instruments::exotics::lookback_option::pricer::LookbackOptionMcPricer::default(),
    );
    registry.register(
        InstrumentType::LookbackOption,
        ModelKey::LookbackBSContinuous,
        crate::instruments::exotics::lookback_option::pricer::LookbackOptionAnalyticalPricer,
    );

    // Quanto Option
    registry.register(
        InstrumentType::QuantoOption,
        ModelKey::QuantoBS,
        crate::instruments::fx::quanto_option::pricer::QuantoOptionAnalyticalPricer,
    );

    // Autocallable

    registry.register(
        InstrumentType::Autocallable,
        ModelKey::MonteCarloGBM,
        crate::instruments::equity::autocallable::pricer::AutocallableMcPricer::default(),
    );

    // CMS Option
    registry.register(
        InstrumentType::CmsOption,
        ModelKey::Black76,
        crate::instruments::rates::cms_option::pricer::CmsOptionPricer::new(),
    );
    registry.register(
        InstrumentType::CmsOption,
        ModelKey::Discounting,
        crate::instruments::rates::cms_option::pricer::CmsOptionPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // CMS Option - Static Replication (Andersen-Piterbarg)
    registry.register(
        InstrumentType::CmsOption,
        ModelKey::StaticReplication,
        crate::instruments::rates::cms_option::replication_pricer::CmsReplicationPricer::new(),
    );

    // CMS Swap
    registry.register(
        InstrumentType::CmsSwap,
        ModelKey::Black76,
        crate::instruments::rates::cms_swap::pricer::CmsSwapPricer::new(),
    );

    // Cliquet Option

    registry.register(
        InstrumentType::CliquetOption,
        ModelKey::MonteCarloGBM,
        crate::instruments::equity::cliquet_option::pricer::CliquetOptionMcPricer::default(),
    );

    // Range Accrual

    registry.register(
        InstrumentType::RangeAccrual,
        ModelKey::MonteCarloGBM,
        crate::instruments::rates::range_accrual::pricer::RangeAccrualMcPricer::default(),
    );

    // Bermudan Swaption LSMC (Hull-White 1F Monte Carlo).
    //
    // Registered with `.require_calibration()`: uncalibrated
    // `HullWhiteParams::default()` (κ=3%, σ=1%) produces 10–30% errors
    // on early-exercise premia. Callers must supply calibrated params
    // via `HullWhiteParams::new(κ, σ)` or a pre-calibrated tree via
    // `with_calibrated_model(...)`; otherwise pricing returns
    // `PricingError::ModelFailure`.

    registry.register(
        InstrumentType::BermudanSwaption,
        ModelKey::MonteCarloHullWhite1F,
        crate::instruments::rates::swaption::pricer::BermudanSwaptionPricer::lsmc_pricer(
            crate::instruments::rates::swaption::pricer::HullWhiteParams::default(),
        )
        .require_calibration(),
    );

    // Bermudan Swaption - Hull-White 1F Tree. See note on the LSMC
    // registration above for the calibration-requirement rationale.
    registry.register(
        InstrumentType::BermudanSwaption,
        ModelKey::HullWhite1F,
        crate::instruments::rates::swaption::pricer::BermudanSwaptionPricer::tree_pricer(
            crate::instruments::rates::swaption::pricer::HullWhiteParams::default(),
        )
        .require_calibration(),
    );

    // Barrier Option - PDE Crank-Nicolson 1D
    registry.register(
        InstrumentType::BarrierOption,
        ModelKey::PdeCrankNicolson1D,
        crate::instruments::exotics::barrier_option::pde_pricer::BarrierOptionPdePricer::default(),
    );

    // Barrier Option - Monte Carlo Heston

    registry.register(
        InstrumentType::BarrierOption,
        ModelKey::MonteCarloHeston,
        crate::instruments::exotics::barrier_option::heston_mc_pricer::BarrierOptionHestonMcPricer::default(),
    );

    // Asian Option - Monte Carlo Heston

    registry.register(
        InstrumentType::AsianOption,
        ModelKey::MonteCarloHeston,
        crate::instruments::exotics::asian_option::heston_mc_pricer::AsianOptionHestonMcPricer::default(),
    );

    // Bermudan Swaption - LMM Monte Carlo

    registry.register(
        InstrumentType::BermudanSwaption,
        ModelKey::LmmMonteCarlo,
        crate::instruments::rates::swaption::lmm_pricer::BermudanSwaptionLmmPricer::default(),
    );

    // Bermudan Swaption - Cheyette Rough Vol Monte Carlo

    registry.register(
        InstrumentType::BermudanSwaption,
        ModelKey::MonteCarloCheyetteRoughVol,
        crate::instruments::rates::swaption::cheyette_rough_pricer::BermudanSwaptionCheyetteRoughPricer::default(),
    );

    // -- Exotic Rate Products --
    //
    // The following instrument types do not have a registered pricer in this
    // function:
    //
    //   * TARN
    //   * Snowball / Inverse Floater
    //   * CMS Spread Option
    //   * Callable Range Accrual
    //
    // Each of these requires a Monte-Carlo, LSMC, or static-replication
    // model that has not been implemented yet. An earlier revision attached
    // a `ModelKey::Discounting` placeholder that routed to
    // `GenericInstrumentPricer::discounting`, which in turn returned
    // `Err(Error::Validation("…MC pricer required…"))`. Advertising a
    // `Discounting` model key for these products was misleading because it
    // implied a working discounting-only fallback.
    //
    // Leaving the registry empty for these `(instrument, model)` pairs
    // produces a clean "no pricer registered" error from the registry. When
    // a real MC/LSM/replication pricer lands, register it here under its
    // actual model key (e.g. `ModelKey::MonteCarloGBM`,
    // `ModelKey::MonteCarloHullWhite1F`, or `ModelKey::StaticReplication`).
}
