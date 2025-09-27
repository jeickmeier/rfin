use crate::instruments::basis_swap::BasisSwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Basis Swap discounting pricer (replaces macro-based version)
pub struct SimpleBasisSwapDiscountingPricer;

impl SimpleBasisSwapDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleBasisSwapDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleBasisSwapDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::BasisSwap, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let basis_swap = instrument.as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::BasisSwap,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(basis_swap.discount_curve_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the instrument's value method
        let pv = basis_swap.value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(basis_swap.id(), as_of, pv))
    }
}
