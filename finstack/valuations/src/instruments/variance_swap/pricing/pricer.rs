use crate::instruments::variance_swap::pricing::engine;
use crate::instruments::variance_swap::VarianceSwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Variance Swap discounting pricer (replaces macro-based version)
pub struct SimpleVarianceSwapDiscountingPricer;

impl SimpleVarianceSwapDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleVarianceSwapDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleVarianceSwapDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::VarianceSwap, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let variance_swap = instrument.as_any()
            .downcast_ref::<VarianceSwap>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::VarianceSwap,
                got: instrument.key()})?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(variance_swap.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = engine::price(variance_swap, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(variance_swap.id(), as_of, pv))
    }
}
