use crate::instruments::cds_index::pricing::engine::CDSIndexPricer;
use crate::instruments::cds_index::CDSIndex;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified CDS Index hazard rate pricer (replaces macro-based version)
pub struct SimpleCdsIndexHazardPricer;

impl SimpleCdsIndexHazardPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleCdsIndexHazardPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleCdsIndexHazardPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CDSIndex, ModelKey::HazardRate)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let cds_index = instrument.as_any()
            .downcast_ref::<CDSIndex>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::CDSIndex,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(cds_index.premium.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = CDSIndexPricer::new().npv(cds_index, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(cds_index.id(), as_of, pv))
    }
}
