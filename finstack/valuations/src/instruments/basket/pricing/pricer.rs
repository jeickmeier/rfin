use crate::instruments::basket::pricing::engine::BasketPricer;
use crate::instruments::basket::Basket;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;


// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Basket discounting pricer (replaces macro-based version)
pub struct SimpleBasketDiscountingPricer;

impl SimpleBasketDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleBasketDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleBasketDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Basket, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let basket = instrument.as_any()
            .downcast_ref::<Basket>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::Basket,
                got: instrument.key(),
            })?;

        // Basket uses epoch as as_of date
        let as_of = Date::from_calendar_date(1970, time::Month::January, 1).unwrap();

        // Compute present value using the engine
        let pv = BasketPricer::new().basket_value(basket, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(basket.id(), as_of, pv))
    }
}
