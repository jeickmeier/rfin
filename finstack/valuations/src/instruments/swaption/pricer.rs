use crate::instruments::common::traits::Instrument;
use crate::instruments::swaption::Swaption;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Swaption pricer supporting multiple models.
pub struct SimpleSwaptionBlackPricer {
    model: ModelKey,
}

impl SimpleSwaptionBlackPricer {
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleSwaptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[finstack_macros::register_pricer]
impl Pricer for SimpleSwaptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Swaption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let swaption = instrument
            .as_any()
            .downcast_ref::<Swaption>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::Swaption,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market
            .get_discount_ref(swaption.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the instrument's value method
        let pv = swaption
            .value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(swaption.id(), as_of, pv))
    }
}

// Auto-register additional Swaption pricer for Discounting model
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(SimpleSwaptionBlackPricer::with_model(crate::pricer::ModelKey::Discounting)),
    }
}
