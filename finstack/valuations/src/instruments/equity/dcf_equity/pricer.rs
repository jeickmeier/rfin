//! DCF pricer implementation.

use super::DiscountedCashFlow;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

/// Pricer for Discounted Cash Flow instruments.
pub struct DcfPricer;

impl Pricer for DcfPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::DCF, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let dcf = instrument
            .as_any()
            .downcast_ref::<DiscountedCashFlow>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::DCF, instrument.key()))?;

        let equity_value = dcf.value(market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(dcf.id(), as_of, equity_value))
    }
}
