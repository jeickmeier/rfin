use crate::instruments::deposit::Deposit;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::Deposit, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let dep: &Deposit = expect_inst(instrument, InstrumentKey::Deposit)?;
        let disc = market.get_ref::<DiscountCurve>(dep.disc_id.clone())?;
        let as_of = disc.base_date();
        use crate::instruments::common::traits::Instrument;
        let pv = dep.value(market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(dep.id.as_str(), as_of, pv))
    }
}


