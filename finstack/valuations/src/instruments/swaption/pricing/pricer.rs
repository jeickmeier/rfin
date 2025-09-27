use crate::instruments::swaption::pricing::engine::SwaptionPricer;
use crate::instruments::swaption::Swaption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;


// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Swaption Black76 pricer (replaces macro-based version)
pub struct SimpleSwaptionBlackPricer;

impl SimpleSwaptionBlackPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleSwaptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleSwaptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Swaption, ModelKey::Black76)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let swaption = instrument.as_any()
            .downcast_ref::<Swaption>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::Swaption,
                got: instrument.key()})?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(swaption.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the swaption pricer
        let pv = SwaptionPricer.npv(swaption, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(swaption.id(), as_of, pv))
    }
}
