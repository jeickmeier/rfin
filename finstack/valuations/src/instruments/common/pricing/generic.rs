//! Generic pricer implementations for instrument pricing.
//!
//! This module provides generic pricer types that eliminate boilerplate by
//! delegating to instruments' `value()` methods. Use these when an instrument
//! implements the [`Instrument`] trait and doesn't need specialized pricing logic.

use crate::instruments::common_impl::traits::Instrument;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use std::marker::PhantomData;

/// Generic pricer for any instrument that implements the Instrument trait.
///
/// This eliminates the need for instrument-specific pricer implementations that just
/// forward to the instrument's `value()` method.
pub struct GenericInstrumentPricer<I> {
    instrument_type: InstrumentType,
    model_key: ModelKey,
    _phantom: PhantomData<I>,
}

impl<I> GenericInstrumentPricer<I>
where
    I: Instrument + 'static,
{
    /// Create a new generic pricer for the specified instrument and model type.
    pub fn new(instrument_type: InstrumentType, model_key: ModelKey) -> Self {
        Self {
            instrument_type,
            model_key,
            _phantom: PhantomData,
        }
    }

    /// Create a generic discounting pricer for the specified instrument type.
    ///
    /// This is a convenience method equivalent to `new(instrument_type, ModelKey::Discounting)`.
    /// Use this when the instrument uses simple cashflow discounting without specialized models.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let irs_pricer = GenericInstrumentPricer::<InterestRateSwap>::discounting(InstrumentType::IRS);
    /// ```
    pub fn discounting(instrument_type: InstrumentType) -> Self {
        Self::new(instrument_type, ModelKey::Discounting)
    }
}

impl<I> Pricer for GenericInstrumentPricer<I>
where
    I: Instrument + 'static,
{
    fn key(&self) -> PricerKey {
        PricerKey::new(self.instrument_type, self.model_key)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let typed_instrument = instrument
            .as_any()
            .downcast_ref::<I>()
            .ok_or_else(|| PricingError::type_mismatch(self.instrument_type, instrument.key()))?;

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument.value(market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed_instrument.id(), as_of, pv))
    }
}

// Special case for CDS which uses HazardRate model
impl GenericInstrumentPricer<crate::instruments::CreditDefaultSwap> {
    /// Create a CDS hazard rate pricer.
    pub fn cds() -> Self {
        Self::new(InstrumentType::CDS, ModelKey::HazardRate)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_pricer_keys() {
        // Test the new discounting() convenience method
        let bond_pricer =
            GenericInstrumentPricer::<crate::instruments::Bond>::discounting(InstrumentType::Bond);
        assert_eq!(
            bond_pricer.key(),
            PricerKey::new(InstrumentType::Bond, ModelKey::Discounting)
        );

        let deposit_pricer = GenericInstrumentPricer::<crate::instruments::Deposit>::discounting(
            InstrumentType::Deposit,
        );
        assert_eq!(
            deposit_pricer.key(),
            PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting)
        );
    }

    #[test]
    fn test_generic_instrument_pricer_with_model_key() {
        // Test that GenericInstrumentPricer works with any model key
        let pricer = GenericInstrumentPricer::<crate::instruments::Bond>::new(
            InstrumentType::Bond,
            ModelKey::HazardRate,
        );
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::Bond, ModelKey::HazardRate)
        );
    }
}
