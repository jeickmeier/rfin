//! Commodity option pricer engine.
//!
//! Provides deterministic PV for `CommodityOption` using Black-76 for
//! European exercise and binomial tree for American exercise.

use crate::instruments::commodity_option::CommodityOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;

/// Commodity option pricer using Black-76 (European) and tree (American).
pub struct CommodityOptionBlackPricer {
    model: ModelKey,
}

impl CommodityOptionBlackPricer {
    /// Create a new commodity option pricer with Black-76 model key.
    pub fn new() -> Self {
        Self {
            model: ModelKey::Black76,
        }
    }

    /// Create a pricer with a specific model key.
    pub fn with_model(model: ModelKey) -> Self {
        Self { model }
    }
}

impl Default for CommodityOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for CommodityOptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CommodityOption, self.model)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let option = instrument
            .as_any()
            .downcast_ref::<CommodityOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CommodityOption, instrument.key())
            })?;

        let pv = option
            .npv(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(option.id(), as_of, pv))
    }
}
