//! Common pricing patterns and shared infrastructure.
//!
//! This module provides generic pricer implementations and shared pricing utilities
//! to eliminate duplication across instrument pricing modules.

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::dates::Date;
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

/// Helper: derive as_of from common discount curves without requiring instrument-specific traits.
fn as_of_from_market<I>(_inst: &I, market: &MarketContext) -> Result<Date, PricingError> {
    if let Ok(disc) = market.get_discount("USD-OIS") {
        Ok(disc.base_date())
    } else if let Ok(disc) = market.get_discount("EUR-ESTR") {
        Ok(disc.base_date())
    } else {
        Err(PricingError::ModelFailure(
            "No suitable discount curve found for as_of date extraction".to_string(),
        ))
    }
}

/// Generic discounting pricer for instruments that can be valued via simple discounting.
///
/// This pricer derives the valuation date from the instrument's configured
/// discount curve and delegates PV calculation to the instrument's `value()`
/// method. It eliminates boilerplate across instrument pricers.
pub struct GenericDiscountingPricer<I> {
    instrument_type: InstrumentType,
    as_of_fn: fn(&I, &MarketContext) -> Result<Date, PricingError>,
    _phantom: PhantomData<I>,
}

impl<I> GenericDiscountingPricer<I>
where
    I: Instrument + 'static,
{
    /// Create a new generic discounting pricer for the specified instrument type.
    pub fn new_discounting_with(
        instrument_type: InstrumentType,
        as_of_fn: fn(&I, &MarketContext) -> Result<Date, PricingError>,
    ) -> Self {
        Self {
            instrument_type,
            as_of_fn,
            _phantom: PhantomData,
        }
    }
}

impl<I> Pricer for GenericDiscountingPricer<I>
where
    I: Instrument + 'static,
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

        // Extract valuation date using configured accessor
        let as_of = (self.as_of_fn)(typed_instrument, market)?;

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument
            .value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed_instrument.id(), as_of, pv))
    }
}

/// Convenience constructor functions for common pricers.
impl GenericDiscountingPricer<crate::instruments::Bond> {
    /// Create a Bond discounting pricer.
    pub fn bond() -> Self {
        Self::new_discounting_with(InstrumentType::Bond, as_of_from_market::<crate::instruments::Bond>)
    }
}

impl GenericDiscountingPricer<crate::instruments::Deposit> {
    /// Create a Deposit discounting pricer.
    pub fn deposit() -> Self {
        Self::new_discounting_with(InstrumentType::Deposit, as_of_from_market::<crate::instruments::Deposit>)
    }
}

impl GenericDiscountingPricer<crate::instruments::ForwardRateAgreement> {
    /// Create a FRA discounting pricer.
    pub fn fra() -> Self {
        Self::new_discounting_with(InstrumentType::FRA, as_of_from_market::<crate::instruments::ForwardRateAgreement>)
    }
}

impl GenericDiscountingPricer<crate::instruments::InterestRateSwap> {
    /// Create an IRS discounting pricer.
    pub fn irs() -> Self {
        Self::new_discounting_with(InstrumentType::IRS, as_of_from_market::<crate::instruments::InterestRateSwap>)
    }
}

impl GenericDiscountingPricer<crate::instruments::Repo> {
    /// Create a Repo discounting pricer.
    pub fn repo() -> Self {
        Self::new_discounting_with(InstrumentType::Repo, as_of_from_market::<crate::instruments::Repo>)
    }
}

impl GenericDiscountingPricer<crate::instruments::Basket> {
    /// Create a Basket discounting pricer.
    pub fn basket() -> Self {
        Self::new_discounting_with(InstrumentType::Basket, as_of_from_market::<crate::instruments::Basket>)
    }
}

impl GenericDiscountingPricer<crate::instruments::VarianceSwap> {
    /// Create a Variance Swap discounting pricer.
    pub fn variance_swap() -> Self {
        Self::new_discounting_with(InstrumentType::VarianceSwap, as_of_from_market::<crate::instruments::VarianceSwap>)
    }
}

impl GenericDiscountingPricer<crate::instruments::InflationLinkedBond> {
    /// Create an Inflation Linked Bond discounting pricer.
    pub fn inflation_linked_bond() -> Self {
        Self::new_discounting_with(
            InstrumentType::InflationLinkedBond,
            as_of_from_market::<crate::instruments::InflationLinkedBond>,
        )
    }
}

impl GenericDiscountingPricer<crate::instruments::FxSwap> {
    /// Create an FX Swap discounting pricer.
    pub fn fx_swap() -> Self {
        Self::new_discounting_with(InstrumentType::FxSwap, as_of_from_market::<crate::instruments::FxSwap>)
    }
}

impl GenericDiscountingPricer<crate::instruments::InflationSwap> {
    /// Create an Inflation Swap discounting pricer.
    pub fn inflation_swap() -> Self {
        Self::new_discounting_with(InstrumentType::InflationSwap, as_of_from_market::<crate::instruments::InflationSwap>)
    }
}

impl GenericDiscountingPricer<crate::instruments::ir_future::InterestRateFuture> {
    /// Create an Interest Rate Future discounting pricer.
    pub fn ir_future() -> Self {
        Self::new_discounting_with(
            InstrumentType::InterestRateFuture,
            as_of_from_market::<crate::instruments::ir_future::InterestRateFuture>,
        )
    }
}

impl GenericDiscountingPricer<crate::instruments::basis_swap::BasisSwap> {
    /// Create a Basis Swap discounting pricer.
    pub fn basis_swap() -> Self {
        Self::new_discounting_with(
            InstrumentType::BasisSwap,
            as_of_from_market::<crate::instruments::basis_swap::BasisSwap>,
        )
    }
}

impl GenericDiscountingPricer<crate::instruments::trs::EquityTotalReturnSwap> {
    /// Create an Equity TRS discounting pricer.
    pub fn equity_trs() -> Self {
        Self::new_discounting_with(
            InstrumentType::TRS,
            as_of_from_market::<crate::instruments::trs::EquityTotalReturnSwap>,
        )
    }
}

impl GenericDiscountingPricer<crate::instruments::trs::FIIndexTotalReturnSwap> {
    /// Create a Fixed Income Index TRS discounting pricer.
    pub fn fi_index_trs() -> Self {
        Self::new_discounting_with(
            InstrumentType::TRS,
            as_of_from_market::<crate::instruments::trs::FIIndexTotalReturnSwap>,
        )
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
