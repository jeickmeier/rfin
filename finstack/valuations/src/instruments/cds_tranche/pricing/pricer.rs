use crate::instruments::cds_tranche::pricing::engine::CDSTranchePricer;
use crate::instruments::cds_tranche::CdsTranche;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified CDS Tranche hazard rate pricer (replaces macro-based version)
pub struct SimpleCdsTrancheHazardPricer;

impl SimpleCdsTrancheHazardPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleCdsTrancheHazardPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleCdsTrancheHazardPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CDSTranche, ModelKey::HazardRate)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let cds_tranche = instrument.as_any()
            .downcast_ref::<CdsTranche>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::CDSTranche,
                got: instrument.key()})?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(cds_tranche.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = CDSTranchePricer::new().price_tranche(cds_tranche, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(cds_tranche.id(), as_of, pv))
    }
}
