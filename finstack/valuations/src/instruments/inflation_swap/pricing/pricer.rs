use crate::instruments::inflation_swap::pricing::engine::InflationSwapPricer;
use crate::instruments::inflation_swap::InflationSwap;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: InflationSwap,
    instrument_key: InflationSwap,
    model: Discounting,
    as_of = |inst: &InflationSwap, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.disc_id)?;
        Ok(disc.base_date())
    },
    pv    = |inst: &InflationSwap, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        let pv_fixed = InflationSwapPricer::new().pv_fixed_leg(inst, market, as_of)?.amount();
        let pv_infl = InflationSwapPricer::new().pv_inflation_leg(inst, market, as_of)?.amount();
        Ok(finstack_core::money::Money::new(pv_infl - pv_fixed, inst.notional.currency()))
    },
);
