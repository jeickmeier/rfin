//! Pricer registrations for rates instruments.
//!
//! Covers: Bond, IRS, FRA, BasisSwap, Deposit, InterestRateFuture, IrFutureOption,
//! BondFuture, CapFloor, Swaption, Repo, DCF.

use super::*;

macro_rules! register_pricer {
    ($registry:expr, $inst:ident, $model:ident, $pricer:expr) => {
        $registry.register_pricer(
            PricerKey::new(InstrumentType::$inst, ModelKey::$model),
            Box::new($pricer),
        );
    };
}

/// Register a minimal set of pricers for rates instruments.
///
/// Intended for environments (like WASM) where registering *all* pricers may be
/// too memory intensive.
pub fn register_rates_pricers(registry: &mut PricerRegistry) {
    // Bond pricers
    register_pricer!(
        registry,
        Bond,
        Discounting,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondDiscountingPricer::default()
    );
    register_pricer!(
        registry,
        Bond,
        HazardRate,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondHazardPricer
    );
    register_pricer!(
        registry,
        Bond,
        Tree,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondOasPricer
    );
    #[cfg(feature = "mc")]
    register_pricer!(
        registry,
        Bond,
        MertonMc,
        crate::instruments::fixed_income::bond::pricing::pricer::SimpleBondMertonMcPricer
    );

    // Interest Rate Swaps
    register_pricer!(
        registry,
        IRS,
        Discounting,
        crate::instruments::common_impl::GenericInstrumentPricer::<
            crate::instruments::InterestRateSwap,
        >::discounting(InstrumentType::IRS)
    );

    // FRA
    register_pricer!(
        registry,
        FRA,
        Discounting,
        crate::instruments::rates::fra::pricer::SimpleFraDiscountingPricer::default()
    );

    // Basis Swap
    register_pricer!(
        registry,
        BasisSwap,
        Discounting,
        crate::instruments::rates::basis_swap::pricer::SimpleBasisSwapDiscountingPricer::default()
    );

    // Deposit
    register_pricer!(
        registry,
        Deposit,
        Discounting,
        crate::instruments::rates::deposit::pricer::SimpleDepositDiscountingPricer::default()
    );

    // Interest Rate Future
    register_pricer!(
        registry,
        InterestRateFuture,
        Discounting,
        crate::instruments::rates::ir_future::pricer::SimpleIrFutureDiscountingPricer::default()
    );

    // IR Future Option
    register_pricer!(
        registry,
        IrFutureOption,
        Discounting,
        crate::instruments::rates::ir_future_option::pricer::IrFutureOptionPricer::default()
    );

    // Bond Future
    register_pricer!(
        registry,
        BondFuture,
        Discounting,
        crate::instruments::fixed_income::bond_future::pricer::BondFuturePricer
    );

    // Cap/Floor
    register_pricer!(
        registry,
        CapFloor,
        Black76,
        crate::instruments::rates::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::default()
    );
    register_pricer!(
        registry,
        CapFloor,
        Discounting,
        crate::instruments::rates::cap_floor::pricing::pricer::SimpleCapFloorBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Swaption
    register_pricer!(
        registry,
        Swaption,
        Black76,
        crate::instruments::rates::swaption::pricer::SimpleSwaptionBlackPricer::default()
    );
    register_pricer!(
        registry,
        Swaption,
        Discounting,
        crate::instruments::rates::swaption::pricer::SimpleSwaptionBlackPricer::with_model(
            ModelKey::Discounting
        )
    );

    // Repo
    register_pricer!(
        registry,
        Repo,
        Discounting,
        crate::instruments::rates::repo::pricer::SimpleRepoDiscountingPricer::default()
    );

    // DCF (Discounted Cash Flow)
    register_pricer!(
        registry,
        DCF,
        Discounting,
        crate::instruments::equity::dcf_equity::pricer::DcfPricer
    );
}
