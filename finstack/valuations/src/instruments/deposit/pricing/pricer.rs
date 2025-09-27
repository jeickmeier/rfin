use crate::instruments::common::traits::Instrument;
use crate::instruments::deposit::Deposit;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;


// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Deposit discounting pricer (replaces macro-based version)
pub struct SimpleDepositDiscountingPricer;

impl SimpleDepositDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleDepositDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleDepositDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let deposit = instrument.as_any()
            .downcast_ref::<Deposit>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::Deposit,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(deposit.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the instrument's value method
        let pv = deposit.value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(deposit.id(), as_of, pv))
    }
}
