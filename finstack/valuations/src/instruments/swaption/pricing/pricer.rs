use crate::instruments::swaption::Swaption;
use crate::instruments::swaption::pricing::engine::SwaptionPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct BlackPricer;

impl BlackPricer { pub fn new() -> Self { Self } }

impl Default for BlackPricer { fn default() -> Self { Self::new() } }

impl Pricer for BlackPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::Swaption, ModelKey::Black76) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let sw: &Swaption = expect_inst(instrument, InstrumentKey::Swaption)?;
        let disc = market.get_ref::<DiscountCurve>(sw.disc_id)?;
        let as_of = disc.base_date();
        let engine = SwaptionPricer;
        let pv = engine.npv(sw, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(sw.id.as_str(), as_of, pv))
    }
}


