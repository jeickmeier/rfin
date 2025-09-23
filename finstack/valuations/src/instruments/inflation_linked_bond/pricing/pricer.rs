use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::instruments::inflation_linked_bond::pricing::engine::InflationLinkedBondEngine;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::InflationLinkedBond, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let ilb: &InflationLinkedBond = expect_inst(instrument, InstrumentKey::InflationLinkedBond)?;
        let disc = market.get_ref::<DiscountCurve>(ilb.disc_id.as_str())?;
        let as_of = disc.base_date();
        let pv = InflationLinkedBondEngine::pv(ilb, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(ilb.id.as_str(), as_of, pv))
    }
}


