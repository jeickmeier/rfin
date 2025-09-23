use crate::instruments::inflation_swap::InflationSwap;
use crate::instruments::inflation_swap::pricing::engine::InflationSwapPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::InflationSwap, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let swap: &InflationSwap = expect_inst(instrument, InstrumentKey::InflationSwap)?;
        let disc = market.get_ref::<DiscountCurve>(swap.disc_id)?;
        let as_of = disc.base_date();
        let pv_fixed = InflationSwapPricer::new().pv_fixed_leg(swap, market, as_of)?.amount();
        let pv_infl = InflationSwapPricer::new().pv_inflation_leg(swap, market, as_of)?.amount();
        let pv = finstack_core::money::Money::new(pv_infl - pv_fixed, swap.notional.currency());
        Ok(crate::results::ValuationResult::stamped(swap.id.as_str(), as_of, pv))
    }
}


