use crate::instruments::cds::pricing::engine::CDSPricer;
use crate::instruments::cds::types::CreditDefaultSwap;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::traits::{Discounting, Survival};

// use macro exported from crate::pricer

crate::impl_dyn_pricer!(
    name: DiscountingPricer,
    instrument: CreditDefaultSwap,
    instrument_key: CDS,
    model: HazardRate,
    as_of = |inst: &CreditDefaultSwap, market: &finstack_core::market_data::MarketContext| -> finstack_core::Result<finstack_core::dates::Date> {
        let disc = market.get_ref::<DiscountCurve>(inst.premium.disc_id.clone())?;
        Ok(disc.base_date())
    },
    pv    = |inst: &CreditDefaultSwap, market: &finstack_core::market_data::MarketContext, as_of: finstack_core::dates::Date| -> finstack_core::Result<finstack_core::money::Money> {
        let disc = market.get_ref::<DiscountCurve>(inst.premium.disc_id.clone())? as &dyn Discounting;
        let surv = market.get_ref::<HazardCurve>(inst.protection.credit_id.clone())? as &dyn Survival;
        CDSPricer::new().npv(inst, disc, surv, as_of)
    },
);
