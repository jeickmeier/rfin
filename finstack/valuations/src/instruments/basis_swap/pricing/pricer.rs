use crate::instruments::basis_swap::BasisSwap;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: BasisSwap,
    instrument_key: BasisSwap,
    model: Discounting,
    as_of = |inst: &BasisSwap, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.discount_curve_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &BasisSwap, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::common::traits::Instrument as _;
        inst.value(market, as_of)
    },
);
