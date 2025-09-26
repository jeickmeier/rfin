use crate::instruments::variance_swap::pricing::engine;
use crate::instruments::variance_swap::VarianceSwap;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: VarianceSwap,
    instrument_key: VarianceSwap,
    model: Discounting,
    as_of = |inst: &VarianceSwap, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &VarianceSwap, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        engine::price(inst, market, as_of)
    },
);
