use crate::instruments::cds::types::CreditDefaultSwap;
use crate::instruments::cds::pricing::engine::CDSPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::CDS, ModelKey::HazardRate) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let cds: &CreditDefaultSwap = expect_inst(instrument, InstrumentKey::CDS)?;
        let disc = market.get_ref::<DiscountCurve>(cds.premium.disc_id)?;
        let surv = market.get_ref::<HazardCurve>(cds.protection.credit_id)?;
        let as_of = disc.base_date();
        use finstack_core::market_data::traits::{Discounting, Survival};
        let pv = CDSPricer::new().npv(cds, disc as &dyn Discounting, surv as &dyn Survival, as_of)?;
        Ok(crate::results::ValuationResult::stamped(cds.id.as_str(), as_of, pv))
    }
}


