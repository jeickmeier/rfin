//! FX touch option pricer implementation.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::fx_touch_option::FxTouchOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

/// FX touch option pricer using Rubinstein-Reiner closed-form.
pub struct SimpleFxTouchOptionPricer {
    model: ModelKey,
}

impl SimpleFxTouchOptionPricer {
    /// Create a new FX touch option pricer with default model.
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create an FX touch option pricer with specified model key.
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleFxTouchOptionPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxTouchOptionPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxTouchOption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<ValuationResult, PricingError> {
        let fx_touch = instrument
            .as_any()
            .downcast_ref::<FxTouchOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::FxTouchOption, instrument.key())
            })?;

        let pv = fx_touch.value(market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(fx_touch.id(), as_of, pv))
    }
}
