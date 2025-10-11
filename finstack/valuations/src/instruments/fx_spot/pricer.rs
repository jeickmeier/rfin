use crate::instruments::common::traits::Instrument;
use crate::instruments::fx_spot::FxSpot;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;

/// Simplified FX Spot discounting pricer that calls the instrument's own methods.
pub struct SimpleFxSpotDiscountingPricer;

impl SimpleFxSpotDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleFxSpotDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxSpotDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxSpot, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let fx_spot = instrument
            .as_any()
            .downcast_ref::<FxSpot>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::FxSpot,
                got: instrument.key(),
            })?;

        // FX Spot uses epoch as as_of date since it's currency conversion, not discounting
        let as_of = Date::from_calendar_date(1970, time::Month::January, 1).unwrap();

        // Use the instrument's own value method
        let pv = fx_spot
            .value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(fx_spot.id(), as_of, pv))
    }
}
