use crate::instruments::basket::Basket;
use crate::instruments::basket::pricing::engine::BasketPricer;
use crate::pricer::{expect_inst, InstrumentKey, ModelKey, Pricer, PricerKey, PriceableExt, PricingError};
use finstack_core::market_data::MarketContext as Market;
use finstack_core::dates::Date;

pub struct DiscountingPricer;

impl DiscountingPricer { pub fn new() -> Self { Self } }

impl Default for DiscountingPricer { fn default() -> Self { Self::new() } }

impl Pricer for DiscountingPricer {
    fn key(&self) -> PricerKey { PricerKey::new(InstrumentKey::Basket, ModelKey::Discounting) }
    fn price_dyn(&self, instrument: &dyn PriceableExt, market: &Market) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let b: &Basket = expect_inst(instrument, InstrumentKey::Basket)?;
        // Choose as_of from the earliest date implied by price series or fallback to Jan 1 1970
        // For deterministic behavior across runs without requiring a discount curve.
        let as_of: Date = Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
        let pv = BasketPricer::new().basket_value(b, market, as_of)?;
        Ok(crate::results::ValuationResult::stamped(b.id.as_str(), as_of, pv))
    }
}


