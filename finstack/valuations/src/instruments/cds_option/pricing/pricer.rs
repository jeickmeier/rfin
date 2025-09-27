use crate::instruments::cds_option::pricing::engine::CdsOptionPricer;
use crate::instruments::cds_option::CdsOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified CDS Option Black76 pricer (replaces macro-based version)
pub struct SimpleCdsOptionBlackPricer;

impl SimpleCdsOptionBlackPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleCdsOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleCdsOptionBlackPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CDSOption, ModelKey::Black76)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let cds_option = instrument.as_any()
            .downcast_ref::<CdsOption>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::CDSOption,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(cds_option.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = CdsOptionPricer::default().npv(cds_option, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(cds_option.id(), as_of, pv))
    }
}
