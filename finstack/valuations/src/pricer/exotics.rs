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
    #[cfg(feature = "mc")]
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
    #[cfg(feature = "mc")]
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
    #[cfg(feature = "mc")]
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
    #[cfg(feature = "mc")]
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
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::CliquetOption,
        ModelKey::MonteCarloGBM,
        crate::instruments::equity::cliquet_option::pricer::CliquetOptionMcPricer::default(),
    );

    // Range Accrual
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::RangeAccrual,
        ModelKey::MonteCarloGBM,
        crate::instruments::rates::range_accrual::pricer::RangeAccrualMcPricer::default(),
    );

    // Bermudan Swaption LSMC (Hull-White 1F Monte Carlo)
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::BermudanSwaption,
        ModelKey::MonteCarloHullWhite1F,
        crate::instruments::rates::swaption::pricer::BermudanSwaptionPricer::lsmc_pricer(
            crate::instruments::rates::swaption::pricer::HullWhiteParams::default(),
        ),
    );

    // Bermudan Swaption - Hull-White 1F Tree
    registry.register(
        InstrumentType::BermudanSwaption,
        ModelKey::HullWhite1F,
        crate::instruments::rates::swaption::pricer::BermudanSwaptionPricer::tree_pricer(
            crate::instruments::rates::swaption::pricer::HullWhiteParams::default(),
        ),
    );

    // Barrier Option - PDE Crank-Nicolson 1D
    registry.register(
        InstrumentType::BarrierOption,
        ModelKey::PdeCrankNicolson1D,
        crate::instruments::exotics::barrier_option::pde_pricer::BarrierOptionPdePricer::default(),
    );

    // Barrier Option - Monte Carlo Heston
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::BarrierOption,
        ModelKey::MonteCarloHeston,
        crate::instruments::exotics::barrier_option::heston_mc_pricer::BarrierOptionHestonMcPricer::default(),
    );

    // Asian Option - Monte Carlo Heston
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::AsianOption,
        ModelKey::MonteCarloHeston,
        crate::instruments::exotics::asian_option::heston_mc_pricer::AsianOptionHestonMcPricer::default(),
    );

    // Bermudan Swaption - LMM Monte Carlo
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::BermudanSwaption,
        ModelKey::LmmMonteCarlo,
        crate::instruments::rates::swaption::lmm_pricer::BermudanSwaptionLmmPricer::default(),
    );

    // Bermudan Swaption - Cheyette Rough Vol Monte Carlo
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::BermudanSwaption,
        ModelKey::MonteCarloCheyetteRoughVol,
        crate::instruments::rates::swaption::cheyette_rough_pricer::BermudanSwaptionCheyetteRoughPricer::default(),
    );
}
