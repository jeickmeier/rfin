use crate::instruments::repo::Repo;
use crate::instruments::repo::pricing::engine::RepoPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::TRS, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let repo: &Repo = expect_inst(instrument, InstrumentKey::TRS)?;
        let disc = market.get_ref::<DiscountCurve>(repo.disc_id)?;
        let as_of = disc.base_date();
        let pv = RepoPricer::new().pv(repo, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(repo.id.as_str(), as_of, pv))
    }
}


