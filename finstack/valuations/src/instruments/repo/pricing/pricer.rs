use crate::instruments::repo::pricing::engine::RepoPricer;
use crate::instruments::repo::Repo;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, PriceableExt, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Repo discounting pricer (replaces macro-based version)
pub struct SimpleRepoDiscountingPricer;

impl SimpleRepoDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleRepoDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleRepoDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Repo, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn PriceableExt,
        market: &MarketContext,
    ) -> std::result::Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let repo = instrument.as_any()
            .downcast_ref::<Repo>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::Repo,
                got: instrument.key(),
            })?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(repo.disc_id)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the engine
        let pv = RepoPricer::new().pv(repo, market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(repo.id(), as_of, pv))
    }
}
