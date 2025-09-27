use crate::instruments::equity_option::pricing::engine;
use crate::instruments::equity_option::EquityOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;


// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Equity Option Black76 pricer (replaces macro-based version)
pub struct SimpleEquityOptionBlackPricer;

impl SimpleEquityOptionBlackPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleEquityOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleEquityOptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::EquityOption, ModelKey::Black76)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let equity_option = instrument.as_any()
            .downcast_ref::<EquityOption>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::EquityOption,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(equity_option.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = engine::npv(equity_option, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(equity_option.id(), as_of, pv))
    }
}
