use crate::instruments::inflation_swap::InflationSwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Inflation Swap discounting pricer (replaces macro-based version)
pub struct SimpleInflationSwapDiscountingPricer;

impl SimpleInflationSwapDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleInflationSwapDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleInflationSwapDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::InflationSwap, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let typed = instrument.as_any()
            .downcast_ref::<InflationSwap>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::InflationSwap,
                got: instrument.key()})?;

        // Get as_of date from the instrument's discount curve
        let disc = market.get_discount_ref(typed.disc_id)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Use instrument's value method
        let pv = typed.value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed.id(), as_of, pv))
    }
}
