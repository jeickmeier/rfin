//! Pricer registrations for exotic instruments.
//!
//! Covers: Basket, AsianOption, BarrierOption, LookbackOption, QuantoOption,
//! Autocallable, CmsOption, CmsSwap, CliquetOption, RangeAccrual, BermudanSwaption.

use super::*;

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register pricers for exotic instruments (barriers, lookbacks, Asians,
/// autocallables, quantos, cliquets, range accruals, Bermudan swaptions).
pub fn register_exotic_pricers(registry: &mut PricerRegistry) {
    // Basket
    register_pricer!(
        registry,
        Basket,
        Discounting,
        crate::instruments::exotics::basket::SimpleBasketDiscountingPricer::default()
    );

    // Asian Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        AsianOption,
        MonteCarloGBM,
        crate::instruments::exotics::asian_option::pricer::AsianOptionMcPricer::default()
    );
    register_pricer!(
        registry,
        AsianOption,
        AsianGeometricBS,
        crate::instruments::exotics::asian_option::pricer::AsianOptionAnalyticalGeometricPricer
    );
    register_pricer!(
        registry,
        AsianOption,
        AsianTurnbullWakeman,
        crate::instruments::exotics::asian_option::pricer::AsianOptionSemiAnalyticalTwPricer
    );

    // Barrier Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        BarrierOption,
        MonteCarloGBM,
        crate::instruments::exotics::barrier_option::pricer::BarrierOptionMcPricer::default()
    );
    register_pricer!(
        registry,
        BarrierOption,
        BarrierBSContinuous,
        crate::instruments::exotics::barrier_option::pricer::BarrierOptionAnalyticalPricer
    );

    // Lookback Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        LookbackOption,
        MonteCarloGBM,
        crate::instruments::exotics::lookback_option::pricer::LookbackOptionMcPricer::default()
    );
    register_pricer!(
        registry,
        LookbackOption,
        LookbackBSContinuous,
        crate::instruments::exotics::lookback_option::pricer::LookbackOptionAnalyticalPricer
    );

    // Quanto Option
    register_pricer!(
        registry,
        QuantoOption,
        QuantoBS,
        crate::instruments::fx::quanto_option::pricer::QuantoOptionAnalyticalPricer
    );

    // Autocallable
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        Autocallable,
        MonteCarloGBM,
        crate::instruments::equity::autocallable::pricer::AutocallableMcPricer::default()
    );

    // CMS Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        CmsOption,
        MonteCarloHullWhite1F,
        crate::instruments::rates::cms_option::pricer::CmsOptionPricer::new()
    );

    // CMS Swap
    register_pricer!(
        registry,
        CmsSwap,
        Black76,
        crate::instruments::rates::cms_swap::pricer::CmsSwapPricer::new()
    );

    // Cliquet Option
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        CliquetOption,
        MonteCarloGBM,
        crate::instruments::equity::cliquet_option::pricer::CliquetOptionMcPricer::default()
    );

    // Range Accrual
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        RangeAccrual,
        MonteCarloGBM,
        crate::instruments::rates::range_accrual::pricer::RangeAccrualMcPricer::default()
    );

    // Bermudan Swaption LSMC (Hull-White 1F Monte Carlo)
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        BermudanSwaption,
        MonteCarloHullWhite1F,
        crate::instruments::rates::swaption::pricer::BermudanSwaptionPricer::lsmc_pricer(
            crate::instruments::rates::swaption::pricer::HullWhiteParams::default()
        )
    );
}
