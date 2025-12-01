use crate::instruments::cap_floor::InterestRateOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Cap/Floor pricer supporting multiple models.
pub struct SimpleCapFloorBlackPricer {
    model: ModelKey,
}

impl SimpleCapFloorBlackPricer {
    /// Create a new cap/floor Black pricer with default model
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create a cap/floor pricer with specified model key
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleCapFloorBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleCapFloorBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CapFloor, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let cap_floor = instrument
            .as_any()
            .downcast_ref::<InterestRateOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CapFloor, instrument.key())
            })?;

        // Use the provided as_of date for consistency
        // Compute present value using the instrument's npv method
        let pv = cap_floor
            .npv(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(cap_floor.id(), as_of, pv))
    }
}
