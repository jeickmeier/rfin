use crate::instruments::repo::Repo;
use crate::instruments::repo::pricing::engine::RepoPricer;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

// use macro exported from crate::pricer

impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: Repo,
    instrument_key: TRS,
    model: Discounting,
    as_of = |inst: &Repo, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.disc_id)?;
        Ok(disc.base_date())
    },
    pv    = |inst: &Repo, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        RepoPricer::new().pv(inst, market, as_of)
    },
);
