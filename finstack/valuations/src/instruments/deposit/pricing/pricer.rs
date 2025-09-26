use crate::instruments::deposit::Deposit;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: Deposit,
    instrument_key: Deposit,
    model: Discounting,
    as_of = |inst: &Deposit, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &Deposit, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::common::traits::Instrument as _;
        inst.value(market, as_of)
    },
);
