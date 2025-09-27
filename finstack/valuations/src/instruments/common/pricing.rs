//! Common pricing patterns and shared infrastructure.
//!
//! This module provides generic pricer implementations and shared pricing utilities
//! to eliminate duplication across instrument pricing modules.

use crate::instruments::common::HasDiscountCurve;
use crate::instruments::common::traits::{Instrument, InstrumentKind};
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
    I: Instrument + HasDiscountCurve + 'static,
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
    I: Instrument + HasDiscountCurve + 'static,
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

        // Derive as_of from the instrument's configured discount curve
        // This eliminates hidden USD/EUR fallbacks and ensures currency safety
        let disc = market
            .get_discount_ref(typed_instrument.discount_curve_id().as_str())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument
            .value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed_instrument.id(), as_of, pv))
    }
}

// Removed USD/EUR heuristic helper; as_of is derived from instrument's own curve

/// Generic discounting pricer for instruments that can be valued via simple discounting.
///
/// This pricer derives the valuation date from the instrument's configured
/// discount curve and delegates PV calculation to the instrument's `value()`
/// method. It eliminates boilerplate across instrument pricers.
pub struct GenericDiscountingPricer<I> {
    instrument_type: InstrumentType,
    _phantom: PhantomData<I>,
}

impl<I> GenericDiscountingPricer<I>
where
    I: Instrument + HasDiscountCurve + InstrumentKind + 'static,
{
    /// Create a new generic discounting pricer. Instrument type is derived from `I`.
    pub fn new() -> Self {
        Self {
            instrument_type: I::TYPE,
            _phantom: PhantomData,
        }
    }
}

impl<I> Pricer for GenericDiscountingPricer<I>
where
    I: Instrument + HasDiscountCurve + InstrumentKind + 'static,
{
    fn key(&self) -> PricerKey {
        PricerKey::new(self.instrument_type, ModelKey::Discounting)
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

        // Extract valuation date from the instrument's configured discount curve
        let disc = market
            .get_discount_ref(typed_instrument.discount_curve_id().as_str())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument
            .value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed_instrument.id(), as_of, pv))
    }
}

// Removed per-instrument constructors; use GenericDiscountingPricer::<I>::new()

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
        let bond_pricer = GenericDiscountingPricer::<crate::instruments::Bond>::new();
        assert_eq!(
            bond_pricer.key(),
            PricerKey::new(InstrumentType::Bond, ModelKey::Discounting)
        );

        let deposit_pricer = GenericDiscountingPricer::<crate::instruments::Deposit>::new();
        assert_eq!(
            deposit_pricer.key(),
            PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting)
        );
    }
}
