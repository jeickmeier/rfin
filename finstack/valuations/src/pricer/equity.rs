//! Pricer registrations for equity instruments.
//!
//! Covers: Equity, EquityOption, EquityTotalReturnSwap, VarianceSwap,
//! EquityIndexFuture, RealEstateAsset, LeveredRealEstateEquity, PrivateMarketsFund.

use super::*;

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register pricers for equity instruments.
pub fn register_equity_pricers(registry: &mut PricerRegistry) {
    // Equity
    register_pricer!(
        registry,
        Equity,
        Discounting,
        crate::instruments::equity::spot::pricer::SimpleEquityDiscountingPricer
    );

    // Equity Option
    register_pricer!(
        registry,
        EquityOption,
        Black76,
        crate::instruments::equity::equity_option::pricer::SimpleEquityOptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        EquityOption,
        Discounting,
        crate::instruments::equity::equity_option::pricer::SimpleEquityOptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        EquityOption,
        HestonFourier,
        crate::instruments::equity::equity_option::pricer::EquityOptionHestonFourierPricer
    );

    // Equity TRS
    register_pricer!(
        registry,
        EquityTotalReturnSwap,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::equity::equity_trs::EquityTotalReturnSwap,
        >::discounting(InstrumentType::EquityTotalReturnSwap)
    );

    // Variance Swap
    register_pricer!(
        registry,
        VarianceSwap,
        Discounting,
        crate::instruments::equity::variance_swap::pricer::SimpleVarianceSwapDiscountingPricer::default()
    );

    // Equity Index Future - uses GenericInstrumentPricer
    register_pricer!(
        registry,
        EquityIndexFuture,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::EquityIndexFuture,
        >::discounting(InstrumentType::EquityIndexFuture)
    );

    // Real Estate Asset - uses GenericInstrumentPricer (curve dependencies)
    register_pricer!(
        registry,
        RealEstateAsset,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::RealEstateAsset,
        >::discounting(InstrumentType::RealEstateAsset)
    );

    // Levered Real Estate Equity - uses GenericInstrumentPricer
    register_pricer!(
        registry,
        LeveredRealEstateEquity,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::LeveredRealEstateEquity,
        >::discounting(InstrumentType::LeveredRealEstateEquity)
    );

    // Private Markets Fund
    register_pricer!(
        registry,
        PrivateMarketsFund,
        Discounting,
        crate::instruments::equity::pe_fund::pricer::PrivateMarketsFundDiscountingPricer
    );
}
