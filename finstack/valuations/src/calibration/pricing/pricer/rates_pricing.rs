//! Rates instrument pricing for `CalibrationPricer`.

use super::CalibrationPricer;
use crate::calibration::pricing::quote_factory::{self, CALIBRATION_NOTIONAL};
use crate::calibration::quotes::RatesQuote;
use crate::instruments::InterestRateSwap;
use finstack_core::money::Money;
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::Currency;
use finstack_core::Result;

impl CalibrationPricer {
    /// Price a rate instrument for calibration (strict when enabled).
    pub fn price_instrument_for_calibration(
        &self,
        quote: &RatesQuote,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        let strict = self.conventions.strict_pricing.unwrap_or(false);
        self.price_with_factory(quote, currency, context, strict)
    }

    // =========================================================================
    // Main Instrument Pricing Dispatch
    // =========================================================================

    /// Price an instrument using the given market context.
    pub fn price_instrument(
        &self,
        quote: &RatesQuote,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        self.price_with_factory(quote, currency, context, false)
    }

    /// Price a rate instrument requiring all conventions to be explicitly provided.
    pub fn price_instrument_strict(
        &self,
        quote: &RatesQuote,
        currency: Currency,
        context: &MarketContext,
    ) -> Result<f64> {
        self.price_with_factory(quote, currency, context, true)
    }

    /// Convenience builder used by tests and validation flows to construct an OIS swap.
    ///
    /// Uses the same factory-backed path as calibration pricing. The provided `notional`
    /// is currently ignored because calibration normalizes by notional; the factory
    /// always uses `CALIBRATION_NOTIONAL`.
    pub fn create_ois_swap(
        &self,
        quote: &RatesQuote,
        _notional: Money,
        currency: Currency,
    ) -> Result<InterestRateSwap> {
        let inst =
            quote_factory::build_instrument_for_rates_quote(self, quote, currency, false)?;
        if let Some(swap) = inst
            .as_ref()
            .as_any()
            .downcast_ref::<InterestRateSwap>()
        {
            return Ok(swap.clone());
        }
        Err(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        ))
    }

    fn price_with_factory(
        &self,
        quote: &RatesQuote,
        currency: Currency,
        context: &MarketContext,
        strict: bool,
    ) -> Result<f64> {
        let inst = quote_factory::build_instrument_for_rates_quote(self, quote, currency, strict)?;
        let pv = inst.value(context, self.base_date)?;
        Ok(pv.amount() / CALIBRATION_NOTIONAL)
    }

}
