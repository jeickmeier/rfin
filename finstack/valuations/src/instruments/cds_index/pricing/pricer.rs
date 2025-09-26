use crate::instruments::cds_index::pricing::engine::CDSIndexPricer;
use crate::instruments::cds_index::CDSIndex;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: CDSIndex,
    instrument_key: CDSIndex,
    model: HazardRate,
    as_of = |inst: &CDSIndex, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.premium.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &CDSIndex, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        CDSIndexPricer::new().npv(inst, market, as_of)
    },
);
