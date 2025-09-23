use crate::instruments::ir_future::InterestRateFuture;
use crate::instruments::ir_future::pricing::engine::IrFutureEngine;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::InterestRateFuture, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let fut: &InterestRateFuture = expect_inst(instrument, InstrumentKey::InterestRateFuture)?;
        let disc = market.get_ref::<DiscountCurve>(fut.disc_id.as_str())?;
        let as_of = disc.base_date();
        let pv = IrFutureEngine::pv(fut, market)?;
        Ok(crate::results::ValuationResult::stamped(fut.id.as_str(), as_of, pv))
    }
}


