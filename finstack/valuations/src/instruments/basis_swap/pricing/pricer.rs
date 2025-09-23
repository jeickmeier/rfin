use crate::instruments::basis_swap::BasisSwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::BasisSwap, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let bs: &BasisSwap = expect_inst(instrument, InstrumentKey::BasisSwap)?;
        let disc = market.get_ref::<DiscountCurve>(bs.discount_curve_id.as_str())?;
        let as_of = disc.base_date();
        let pv = bs.value(market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(bs.id.as_str(), as_of, pv))
    }
}


