use crate::instruments::irs::pricing::engine::IrsEngine;
use crate::instruments::irs::types::InterestRateSwap;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: InterestRateSwap,
    instrument_key: IRS,
    model: Discounting,
    as_of = |inst: &InterestRateSwap, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.fixed.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &InterestRateSwap, market: &finstack_core::market_data::MarketContext, _as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        IrsEngine::pv(inst, market)
    },
);
