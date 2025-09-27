use crate::instruments::fx_option::FxOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified FX Option Black76 pricer (replaces macro-based version)
pub struct SimpleFxOptionBlackPricer;

impl SimpleFxOptionBlackPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleFxOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxOptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxOption, ModelKey::Black76)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let fx_option = instrument.as_any()
            .downcast_ref::<FxOption>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::FxOption,
                got: instrument.key()})?;

        // Get as_of date from domestic discount curve
        let disc = market.get_discount_ref(fx_option.domestic_disc_id.as_str())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Use instrument's value method
        let pv = fx_option.value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(fx_option.id(), as_of, pv))
    }
}
