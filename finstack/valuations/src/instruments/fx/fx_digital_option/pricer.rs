//! FX digital option pricer implementation.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_digital_option::FxDigitalOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

/// FX digital option pricer using Garman-Kohlhagen adapted closed-form.
pub struct SimpleFxDigitalOptionPricer {
    model: ModelKey,
}

impl SimpleFxDigitalOptionPricer {
    /// Create a new FX digital option pricer with default model.
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create an FX digital option pricer with specified model key.
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleFxDigitalOptionPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxDigitalOptionPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxDigitalOption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<ValuationResult, PricingError> {
        let fx_digital = instrument
            .as_any()
            .downcast_ref::<FxDigitalOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxDigitalOption, instrument.key())
            })?;

        let pv = fx_digital.value(market, as_of).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(fx_digital.id(), as_of, pv))
    }
}
