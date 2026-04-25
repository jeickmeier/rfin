//! Vanilla swaption pricer implementation.

use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::swaption::Swaption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

/// Swaption pricer supporting Black-76 and instrument-selected fallback models.
pub struct SimpleSwaptionBlackPricer {
    model: ModelKey,
}

impl SimpleSwaptionBlackPricer {
    /// Create a new swaption pricer with the default Black-76 model.
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create a swaption pricer with the specified model key.
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleSwaptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleSwaptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Swaption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let swaption = instrument
            .as_any()
            .downcast_ref::<Swaption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::Swaption, instrument.key())
            })?;

        let pv = match self.model {
            ModelKey::Black76 => {
                if swaption.sabr_params.is_some() {
                    swaption.price_sabr(market, as_of).map_err(|e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    })?
                } else {
                    let strike = swaption.strike_f64().map_err(|e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    })?;
                    let time_to_expiry = year_fraction(swaption.day_count, as_of, swaption.expiry)
                        .map_err(|e| {
                            PricingError::model_failure_with_context(
                                e.to_string(),
                                PricingErrorContext::default(),
                            )
                        })?;

                    let vol_provider = market
                        .get_vol_provider(swaption.vol_surface_id.as_str())
                        .map_err(|e| {
                            PricingError::missing_market_data_with_context(
                                e.to_string(),
                                PricingErrorContext::default(),
                            )
                        })?;

                    let underlying_tenor =
                        year_fraction(swaption.day_count, swaption.expiry, swaption.swap_end)
                            .map_err(|e| {
                                PricingError::model_failure_with_context(
                                    e.to_string(),
                                    PricingErrorContext::default(),
                                )
                            })?;

                    let vol = if let Some(impl_vol) =
                        swaption.pricing_overrides.market_quotes.implied_volatility
                    {
                        impl_vol
                    } else {
                        vol_provider.vol_clamped(time_to_expiry, underlying_tenor, strike)
                    };

                    swaption.price_black(market, vol, as_of).map_err(|e| {
                        PricingError::model_failure_with_context(
                            e.to_string(),
                            PricingErrorContext::default(),
                        )
                    })?
                }
            }
            _ => swaption.value(market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?,
        };

        Ok(ValuationResult::stamped(swaption.id(), as_of, pv))
    }
}
