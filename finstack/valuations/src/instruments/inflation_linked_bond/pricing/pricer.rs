use crate::instruments::inflation_linked_bond::pricing::engine::InflationLinkedBondEngine;
use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Inflation Linked Bond discounting pricer (replaces macro-based version)
pub struct SimpleInflationLinkedBondDiscountingPricer;

impl SimpleInflationLinkedBondDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleInflationLinkedBondDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleInflationLinkedBondDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::InflationLinkedBond, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let inflation_bond = instrument.as_any()
            .downcast_ref::<InflationLinkedBond>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::InflationLinkedBond,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(inflation_bond.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = InflationLinkedBondEngine::pv(inflation_bond, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(inflation_bond.id(), as_of, pv))
    }
}
