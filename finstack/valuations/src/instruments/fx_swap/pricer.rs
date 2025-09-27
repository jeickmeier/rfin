use crate::instruments::fx_swap::FxSwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified FX Swap discounting pricer (replaces macro-based version)
pub struct SimpleFxSwapDiscountingPricer;

impl SimpleFxSwapDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleFxSwapDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleFxSwapDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::FxSwap, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let fx_swap = instrument.as_any()
            .downcast_ref::<FxSwap>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::FxSwap,
                got: instrument.key()})?;

        // Get as_of date from domestic discount curve
        let disc = market.get_discount_ref(fx_swap.domestic_disc_id)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Use instrument's value method
        let pv = fx_swap.value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(fx_swap.id(), as_of, pv))
    }
}
