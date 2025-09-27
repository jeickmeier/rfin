use crate::instruments::inflation_swap::pricing::engine::InflationSwapPricer;
use crate::instruments::inflation_swap::InflationSwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Inflation Swap discounting pricer (replaces macro-based version)
pub struct SimpleInflationSwapDiscountingPricer;

impl SimpleInflationSwapDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleInflationSwapDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleInflationSwapDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::InflationSwap, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let inflation_swap = instrument.as_any()
            .downcast_ref::<InflationSwap>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::InflationSwap,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(inflation_swap.disc_id)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine (fixed leg - inflation leg)
        let pricer = InflationSwapPricer::new();
        let pv_fixed = pricer.pv_fixed_leg(inflation_swap, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?
            .amount();
        let pv_infl = pricer.pv_inflation_leg(inflation_swap, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?
            .amount();
        
        let pv = finstack_core::money::Money::new(
            pv_infl - pv_fixed, 
            inflation_swap.notional.currency()
        );

        // Return stamped result
        Ok(ValuationResult::stamped(inflation_swap.id(), as_of, pv))
    }
}
