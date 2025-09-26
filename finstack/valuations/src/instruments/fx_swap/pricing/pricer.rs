use crate::instruments::fx_swap::pricing::engine::FxSwapPricer;
use crate::instruments::fx_swap::FxSwap;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: FxSwap,
    instrument_key: FxSwap,
    model: Discounting,
    as_of = |inst: &FxSwap, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.domestic_disc_id)?;
        Ok(disc.base_date())
    },
    pv    = |inst: &FxSwap, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        FxSwapPricer::pv(inst, market, as_of)
    },
);
