use crate::instruments::fx_swap::FxSwap;
use crate::instruments::fx_swap::pricing::engine::FxSwapPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::FxSwap, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let swap: &FxSwap = expect_inst(instrument, InstrumentKey::FxSwap)?;
        let disc = market.get_ref::<DiscountCurve>(swap.domestic_disc_id)?;
        let as_of = disc.base_date();
        let pv = FxSwapPricer::pv(swap, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(swap.id.as_str(), as_of, pv))
    }
}


