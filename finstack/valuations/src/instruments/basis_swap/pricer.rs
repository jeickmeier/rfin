//! Basis swap pricer implementation.
//!
//! This module provides a simple pricer that properly extracts the as_of date from
//! the instrument's specified discount curve and delegates to the instrument's own
//! pricing methods.

use crate::instruments::basis_swap::BasisSwap;
use crate::instruments::common::{traits::Instrument, HasDiscountCurve};
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

/// Basis swap discounting pricer.
///
/// This pricer handles basis swap valuation using discounting methodology.
/// It extracts the valuation date from the instrument's specified discount curve
/// and delegates the actual pricing to the instrument's built-in methods.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleBasisSwapDiscountingPricer;

impl SimpleBasisSwapDiscountingPricer {
    /// Create a new basis swap discounting pricer.
    pub const fn new() -> Self {
        Self
    }
}

impl Pricer for SimpleBasisSwapDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::BasisSwap, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let basis_swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::BasisSwap,
                got: instrument.key(),
            })?;

        // Get as_of date from the instrument's specified discount curve
        // This is the correct approach for basis swaps which have a specific discount curve
        let disc = market
            .get_discount_ref(basis_swap.discount_curve_id().clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the instrument's value method
        let pv = basis_swap
            .value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(basis_swap.id(), as_of, pv))
    }
}
