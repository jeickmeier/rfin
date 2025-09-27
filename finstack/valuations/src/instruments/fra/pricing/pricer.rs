use crate::instruments::fra::pricing::engine::FraEngine;
use crate::instruments::fra::ForwardRateAgreement;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified FRA discounting pricer (replaces macro-based version)
pub struct SimpleFraDiscountingPricer;

impl SimpleFraDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleFraDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFraDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FRA, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let fra = instrument.as_any()
            .downcast_ref::<ForwardRateAgreement>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::FRA,
                got: instrument.key()})?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(fra.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = FraEngine::pv(fra, market)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(fra.id(), as_of, pv))
    }
}
