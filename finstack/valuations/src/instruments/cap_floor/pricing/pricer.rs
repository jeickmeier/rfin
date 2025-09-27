use crate::instruments::cap_floor::InterestRateOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Cap/Floor Black76 pricer (replaces macro-based version)
pub struct SimpleCapFloorBlackPricer;

impl SimpleCapFloorBlackPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleCapFloorBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleCapFloorBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CapFloor, ModelKey::Black76)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let cap_floor = instrument.as_any()
            .downcast_ref::<InterestRateOption>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::CapFloor,
                got: instrument.key()})?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(cap_floor.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the instrument's npv method
        let pv = cap_floor.npv(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(cap_floor.id(), as_of, pv))
    }
}
