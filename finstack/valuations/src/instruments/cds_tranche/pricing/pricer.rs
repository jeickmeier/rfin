use crate::instruments::cds_tranche::pricing::engine::CDSTranchePricer;
use crate::instruments::cds_tranche::CdsTranche;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: CdsTranche,
    instrument_key: CDSTranche,
    model: HazardRate,
    as_of = |inst: &CdsTranche, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_discount_ref(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &CdsTranche, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        CDSTranchePricer::new().price_tranche(inst, market, as_of)
    },
);
