use crate::instruments::fra::pricing::engine::FraEngine;
use crate::instruments::fra::ForwardRateAgreement;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: ForwardRateAgreement,
    instrument_key: IRS,
    model: Discounting,
    as_of = |inst: &ForwardRateAgreement, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &ForwardRateAgreement, market: &finstack_core::market_data::MarketContext, _as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        FraEngine::pv(inst, market)
    },
);
