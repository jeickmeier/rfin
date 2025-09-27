//! Common pricing patterns and shared infrastructure.
//!
//! This module provides generic pricer implementations and shared pricing utilities
//! to eliminate duplication across instrument pricing modules.

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
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
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let typed_instrument = instrument
            .as_any()
            .downcast_ref::<I>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: self.instrument_type,
                got: instrument.key(),
            })?;

        // Get as_of date by trying to extract from an available discount curve
        // This is a simplified approach - in a real system you'd want more sophisticated curve discovery
        let as_of = if let Ok(disc) = market.get_discount("USD-OIS") {
            disc.base_date()
        } else if let Ok(disc) = market.get_discount("EUR-ESTR") {
            disc.base_date()
        } else {
            // Use a fallback - this could be enhanced to be more robust
            return Err(PricingError::ModelFailure(
                "No suitable discount curve found for as_of date extraction".to_string(),
            ));
        };

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument
            .value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed_instrument.id(), as_of, pv))
    }
}

/// Generic discounting pricer for any instrument that implements the Instrument trait.
///
/// This is a convenience type alias for GenericInstrumentPricer with ModelKey::Discounting.
pub type GenericDiscountingPricer<I> = GenericInstrumentPricer<I>;

impl<I> GenericDiscountingPricer<I>
where
    I: Instrument + 'static,
{
    /// Create a new generic discounting pricer for the specified instrument type.
    pub fn new_discounting(instrument_type: InstrumentType) -> Self {
        Self::new(instrument_type, ModelKey::Discounting)
    }
}

/// Convenience constructor functions for common pricers.
impl GenericDiscountingPricer<crate::instruments::Bond> {
    /// Create a Bond discounting pricer.
    pub fn bond() -> Self {
        Self::new_discounting(InstrumentType::Bond)
    }
}

impl GenericDiscountingPricer<crate::instruments::Deposit> {
    /// Create a Deposit discounting pricer.
    pub fn deposit() -> Self {
        Self::new_discounting(InstrumentType::Deposit)
    }
}

impl GenericDiscountingPricer<crate::instruments::ForwardRateAgreement> {
    /// Create a FRA discounting pricer.
    pub fn fra() -> Self {
        Self::new_discounting(InstrumentType::FRA)
    }
}

impl GenericDiscountingPricer<crate::instruments::InterestRateSwap> {
    /// Create an IRS discounting pricer.
    pub fn irs() -> Self {
        Self::new_discounting(InstrumentType::IRS)
    }
}

impl GenericDiscountingPricer<crate::instruments::Repo> {
    /// Create a Repo discounting pricer.
    pub fn repo() -> Self {
        Self::new_discounting(InstrumentType::Repo)
    }
}

impl GenericDiscountingPricer<crate::instruments::Basket> {
    /// Create a Basket discounting pricer.
    pub fn basket() -> Self {
        Self::new_discounting(InstrumentType::Basket)
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
mod tests {
    use super::*;

    #[test]
    fn test_generic_pricer_keys() {
        let bond_pricer = GenericDiscountingPricer::<crate::instruments::Bond>::bond();
        assert_eq!(
            bond_pricer.key(),
            PricerKey::new(InstrumentType::Bond, ModelKey::Discounting)
        );

        let deposit_pricer = GenericDiscountingPricer::<crate::instruments::Deposit>::deposit();
        assert_eq!(
            deposit_pricer.key(),
            PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting)
        );
    }
}
