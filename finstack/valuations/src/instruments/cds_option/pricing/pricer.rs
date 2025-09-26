use crate::instruments::cds_option::pricing::engine::CdsOptionPricer;
use crate::instruments::cds_option::CdsOption;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: BlackPricer,
    instrument: CdsOption,
    instrument_key: CDSOption,
    model: Black76,
    as_of = |inst: &CdsOption, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &CdsOption, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        CdsOptionPricer::default().npv(inst, market, as_of)
    },
);
