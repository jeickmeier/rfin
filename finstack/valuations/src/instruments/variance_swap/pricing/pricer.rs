use crate::instruments::variance_swap::VarianceSwap;
use crate::instruments::variance_swap::pricing::engine;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::VarianceSwap, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let vs: &VarianceSwap = expect_inst(instrument, InstrumentKey::VarianceSwap)?;
        let disc = market.get_ref::<DiscountCurve>(vs.disc_id.as_str())?;
        let as_of = disc.base_date();
        let pv = engine::price(vs, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(vs.id.as_str(), as_of, pv))
    }
}


