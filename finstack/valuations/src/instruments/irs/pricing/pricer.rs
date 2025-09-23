use crate::instruments::irs::types::InterestRateSwap;
use crate::instruments::irs::pricing::engine::IrsEngine;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::IRS, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let irs: &InterestRateSwap = expect_inst(instrument, InstrumentKey::IRS)?;
        let disc = market.get_ref::<DiscountCurve>(irs.fixed.disc_id)?;
        let as_of = disc.base_date();
        let pv = IrsEngine::pv(irs, market)?;
        Ok(crate::results::ValuationResult::stamped(irs.id.as_str(), as_of, pv))
    }
}


