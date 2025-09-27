use crate::instruments::irs::pricing::engine::IrsEngine;
use crate::instruments::irs::types::InterestRateSwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;


// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified IRS discounting pricer (replaces macro-based version)
pub struct SimpleIrsDiscountingPricer;

impl SimpleIrsDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleIrsDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleIrsDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::IRS, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let irs = instrument.as_any()
            .downcast_ref::<InterestRateSwap>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::IRS,
                got: instrument.key()})?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(irs.fixed.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = IrsEngine::pv(irs, market)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(irs.id(), as_of, pv))
    }
}
