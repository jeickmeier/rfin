//! Pricer registrations for rates instruments.
//!
//! Covers: Bond, IRS, FRA, BasisSwap, Deposit, InterestRateFuture, IrFutureOption,
//! BondFuture, CapFloor, Swaption, Repo, DCF.

use super::{InstrumentType, ModelKey, PricerRegistry};

/// Register a minimal set of pricers for rates instruments.
///
/// Intended for environments (like WASM) where registering *all* pricers may be
/// too memory intensive.
pub fn register_rates_pricers(registry: &mut PricerRegistry) {
    // Bond pricers
    registry.register(
        InstrumentType::Bond,
        ModelKey::Discounting,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondDiscountingPricer::default(),
    );
    registry.register(
        InstrumentType::Bond,
        ModelKey::HazardRate,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondHazardPricer,
    );
    registry.register(
        InstrumentType::Bond,
        ModelKey::Tree,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondOasPricer,
    );
    #[cfg(feature = "mc")]
    registry.register(
        InstrumentType::Bond,
        ModelKey::MertonMc,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondMertonMcPricer,
    );

    // Interest Rate Swaps
    registry.register(
        InstrumentType::IRS,
        ModelKey::Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::InterestRateSwap,
        >::discounting(InstrumentType::IRS),
    );

    // FRA
    registry.register(
        InstrumentType::FRA,
        ModelKey::Discounting,
        crate::instruments::rates::fra::pricer::SimpleFraDiscountingPricer::default(),
    );

    // Basis Swap
    registry.register(
        InstrumentType::BasisSwap,
        ModelKey::Discounting,
        crate::instruments::rates::basis_swap::pricer::SimpleBasisSwapDiscountingPricer::default(),
    );

    // Deposit
    registry.register(
        InstrumentType::Deposit,
        ModelKey::Discounting,
        crate::instruments::rates::deposit::pricer::SimpleDepositDiscountingPricer::default(),
    );

    // Interest Rate Future
    registry.register(
        InstrumentType::InterestRateFuture,
        ModelKey::Discounting,
        crate::instruments::rates::ir_future::pricer::SimpleIrFutureDiscountingPricer::default(),
    );

    // IR Future Option
    registry.register(
        InstrumentType::IrFutureOption,
        ModelKey::Discounting,
        crate::instruments::rates::ir_future_option::pricer::IrFutureOptionPricer::default(),
    );

    // Bond Future
    registry.register(
        InstrumentType::BondFuture,
        ModelKey::Discounting,
        crate::instruments::fixed_income::bond_future::pricer::BondFuturePricer,
    );

    // Cap/Floor
    registry.register(
        InstrumentType::CapFloor,
        ModelKey::Black76,
        crate::instruments::rates::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::default(),
    );
    registry.register(
        InstrumentType::CapFloor,
        ModelKey::Discounting,
        crate::instruments::rates::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // Swaption
    registry.register(
        InstrumentType::Swaption,
        ModelKey::Black76,
        crate::instruments::rates::swaption::pricer::SimpleSwaptionBlackPricer::default(),
    );
    registry.register(
        InstrumentType::Swaption,
        ModelKey::Discounting,
        crate::instruments::rates::swaption::pricer::SimpleSwaptionBlackPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    // Repo
    registry.register(
        InstrumentType::Repo,
        ModelKey::Discounting,
        crate::instruments::rates::repo::pricer::SimpleRepoDiscountingPricer::default(),
    );

    // DCF (Discounted Cash Flow)
    registry.register(
        InstrumentType::DCF,
        ModelKey::Discounting,
        crate::instruments::equity::dcf_equity::pricer::DcfPricer,
    );
}
