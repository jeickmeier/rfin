use crate::instruments::inflation_linked_bond::pricing::engine::InflationLinkedBondEngine;
use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: InflationLinkedBond,
    instrument_key: InflationLinkedBond,
    model: Discounting,
    as_of = |inst: &InflationLinkedBond, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &InflationLinkedBond, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        InflationLinkedBondEngine::pv(inst, market, as_of)
    },
);
