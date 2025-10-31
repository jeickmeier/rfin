//! Common pricing patterns and shared infrastructure.
//!
//! This module provides generic pricer implementations and shared pricing utilities
//! to eliminate duplication across instrument pricing modules.

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::types::CurveId;
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
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let typed_instrument =
            instrument
                .as_any()
                .downcast_ref::<I>()
                .ok_or_else(|| PricingError::type_mismatch(
                    self.instrument_type,
                    instrument.key(),
                ))?;

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed_instrument.id(), as_of, pv))
    }
}

// Removed USD/EUR heuristic helper; as_of is derived from instrument's own curve

/// Trait for instruments with a primary discount curve.
///
/// This trait is used by generic pricers and metric calculators to extract
/// discount curve IDs. All instruments with a discount curve should implement this.
///
/// **Note**: This is primarily an internal helper trait. End-users typically
/// don't need to interact with it directly.
pub trait HasDiscountCurve {
    /// Get the instrument's primary discount curve ID.
    fn discount_curve_id(&self) -> &CurveId;
}

/// Generic discounting pricer for instruments that can be valued via simple discounting.
///
/// This pricer derives the valuation date from the instrument's discount curve
/// and delegates PV calculation to the instrument's `value()` method.
/// It eliminates boilerplate across instrument pricers.
pub struct GenericDiscountingPricer<I> {
    instrument_type: InstrumentType,
    _phantom: PhantomData<I>,
}

impl<I> GenericDiscountingPricer<I>
where
    I: Instrument + HasDiscountCurve + 'static,
{
    /// Create a new generic discounting pricer for the specified instrument type.
    pub fn new(instrument_type: InstrumentType) -> Self {
        Self {
            instrument_type,
            _phantom: PhantomData,
        }
    }
}

impl<I> Pricer for GenericDiscountingPricer<I>
where
    I: Instrument + HasDiscountCurve + 'static,
{
    fn key(&self) -> PricerKey {
        PricerKey::new(self.instrument_type, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let typed_instrument =
            instrument
                .as_any()
                .downcast_ref::<I>()
                .ok_or_else(|| PricingError::type_mismatch(
                    self.instrument_type,
                    instrument.key(),
                ))?;

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

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
        let bond_pricer =
            GenericDiscountingPricer::<crate::instruments::Bond>::new(InstrumentType::Bond);
        assert_eq!(
            bond_pricer.key(),
            PricerKey::new(InstrumentType::Bond, ModelKey::Discounting)
        );

        let deposit_pricer =
            GenericDiscountingPricer::<crate::instruments::Deposit>::new(InstrumentType::Deposit);
        assert_eq!(
            deposit_pricer.key(),
            PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting)
        );
    }
}
