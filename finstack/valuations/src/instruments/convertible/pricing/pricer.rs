use crate::instruments::convertible::pricing::engine::{
    price_convertible_bond, ConvertibleTreeType,
};
use crate::instruments::convertible::ConvertibleBond;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Convertible Bond discounting pricer (replaces macro-based version)
pub struct SimpleConvertibleDiscountingPricer;

impl SimpleConvertibleDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleConvertibleDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleConvertibleDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Convertible, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let convertible = instrument.as_any()
            .downcast_ref::<ConvertibleBond>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::Convertible,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(convertible.disc_id)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine with binomial tree
        let pv = price_convertible_bond(convertible, market, ConvertibleTreeType::Binomial(100))
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(convertible.id(), as_of, pv))
    }
}
