use crate::instruments::cds::pricing::engine::CDSPricer;
use crate::instruments::cds::types::CreditDefaultSwap;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::traits::{Discounting, Survival};
use finstack_core::market_data::MarketContext;


// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified CDS hazard rate pricer (replaces macro-based version)
pub struct SimpleCdsDiscountingPricer;

impl SimpleCdsDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleCdsDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleCdsDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CDS, ModelKey::HazardRate)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let cds = instrument.as_any()
            .downcast_ref::<CreditDefaultSwap>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::CDS,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(cds.premium.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Get survival curve
        let disc_trait = disc as &dyn Discounting;
        let surv = market.get_hazard_ref(cds.protection.credit_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let surv_trait = surv as &dyn Survival;

        // Compute present value using the engine
        let pv = CDSPricer::new().npv(cds, disc_trait, surv_trait, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(cds.id(), as_of, pv))
    }
}
