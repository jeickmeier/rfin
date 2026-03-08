//! Pricer registrations for equity instruments.
//!
//! Covers: Equity, EquityOption, EquityTotalReturnSwap, VarianceSwap,
//! EquityIndexFuture, RealEstateAsset, LeveredRealEstateEquity, PrivateMarketsFund.

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for equity instruments.
pub fn register_equity_pricers(registry: &mut PricerRegistry) {
    // Equity
    registry.register(
        InstrumentType::Equity,
        ModelKey::Discounting,
        crate::instruments::equity::spot::pricer::SimpleEquityDiscountingPricer,
    );

    // Equity Option
    registry.register(
        InstrumentType::EquityOption,
        ModelKey::Black76,
        crate::instruments::equity::equity_option::pricer::SimpleEquityOptionBlackPricer::default(),
    );
    registry.register(
        InstrumentType::EquityOption,
        ModelKey::Discounting,
        crate::instruments::equity::equity_option::pricer::SimpleEquityOptionBlackPricer::with_model(
            ModelKey::Discounting,
        ),
    );
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::EquityOption,
        ModelKey::HestonFourier,
        crate::instruments::equity::equity_option::pricer::EquityOptionHestonFourierPricer,
    );

    // Equity TRS
    registry.register(
        InstrumentType::EquityTotalReturnSwap,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::equity::equity_trs::EquityTotalReturnSwap,
        >::discounting(InstrumentType::EquityTotalReturnSwap),
    );

    // Variance Swap
    registry.register(
        InstrumentType::VarianceSwap,
        ModelKey::Discounting,
        crate::instruments::equity::variance_swap::pricer::SimpleVarianceSwapDiscountingPricer::default(),
    );

    // Equity Index Future - uses GenericInstrumentPricer
    registry.register(
        InstrumentType::EquityIndexFuture,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::EquityIndexFuture,
        >::discounting(InstrumentType::EquityIndexFuture),
    );

    // Real Estate Asset - uses GenericInstrumentPricer (curve dependencies)
    registry.register(
        InstrumentType::RealEstateAsset,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::RealEstateAsset,
        >::discounting(InstrumentType::RealEstateAsset),
    );

    // Levered Real Estate Equity - uses GenericInstrumentPricer
    registry.register(
        InstrumentType::LeveredRealEstateEquity,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::LeveredRealEstateEquity,
        >::discounting(InstrumentType::LeveredRealEstateEquity),
    );

    // Private Markets Fund
    registry.register(
        InstrumentType::PrivateMarketsFund,
        ModelKey::Discounting,
        crate::instruments::equity::pe_fund::pricer::PrivateMarketsFundDiscountingPricer,
    );
}
