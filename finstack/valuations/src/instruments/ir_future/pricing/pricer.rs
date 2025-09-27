use crate::instruments::ir_future::pricing::engine::IrFutureEngine;
use crate::instruments::ir_future::InterestRateFuture;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Interest Rate Future discounting pricer (replaces macro-based version)
pub struct SimpleIrFutureDiscountingPricer;

impl SimpleIrFutureDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleIrFutureDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleIrFutureDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::InterestRateFuture, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let ir_future = instrument.as_any()
            .downcast_ref::<InterestRateFuture>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::InterestRateFuture,
                got: instrument.key()})?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(ir_future.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = IrFutureEngine::pv(ir_future, market)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(ir_future.id(), as_of, pv))
    }
}
