use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_option::FxOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified FX Option Black76 pricer (replaces macro-based version)
pub struct SimpleFxOptionBlackPricer {
    model: ModelKey,
}

impl SimpleFxOptionBlackPricer {
    /// Create a new FX option pricer with default model
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create an FX option pricer with specified model key
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleFxOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxOptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxOption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let fx_option = instrument
            .as_any()
            .downcast_ref::<FxOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxOption, instrument.key())
            })?;

        // Use the provided as_of date for consistency
        // Use instrument's value method
        let pv = fx_option.value(market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        // Return stamped result
        Ok(ValuationResult::stamped(fx_option.id(), as_of, pv))
    }
}
