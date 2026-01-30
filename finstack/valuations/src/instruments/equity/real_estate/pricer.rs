//! Real estate asset pricer implementation.

use super::RealEstateAsset;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

/// Pricer for real estate assets (DCF/direct cap).
pub struct RealEstateAssetDiscountingPricer;

impl Pricer for RealEstateAssetDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RealEstateAsset, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let asset = instrument
            .as_any()
            .downcast_ref::<RealEstateAsset>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::RealEstateAsset, instrument.key())
            })?;

        let value = asset.value(market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(asset.id(), as_of, value))
    }
}
