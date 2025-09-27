use crate::instruments::bond::pricing::tree_pricer::TreePricer;
use crate::instruments::bond::types::Bond;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;

use indexmap::IndexMap;


pub struct OasPricer;

impl OasPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OasPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for OasPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(
            crate::pricer::InstrumentType::Bond,
            crate::pricer::ModelKey::Tree,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &finstack_core::market_data::MarketContext,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        // Use the new simplified OAS pricer
        let oas_pricer = SimpleBondOasPricer::new();
        oas_pricer.price_dyn(instrument, market)
    }
}

// ========================= NEW SIMPLIFIED PRICERS =========================

/// New simplified Bond discounting pricer (replaces macro-based version)
pub struct SimpleBondDiscountingPricer;

impl SimpleBondDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleBondDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleBondDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Bond, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting using the new system
        let bond = instrument.as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::Bond,
                got: instrument.key()})?;

        // Get as_of date from discount curve
        let disc = market.get_discount_ref(bond.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Compute present value using the instrument's value method
        let pv = bond.value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(bond.id(), as_of, pv))
    }
}

/// New simplified Bond OAS pricer (replaces macro-based version)  
pub struct SimpleBondOasPricer;

impl SimpleBondOasPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleBondOasPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleBondOasPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Bond, ModelKey::Tree)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let bond = instrument.as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::TypeMismatch {
                expected: InstrumentType::Bond,
                got: instrument.key()})?;

        // Get as_of date
        let disc = market.get_discount_ref(bond.disc_id.clone())
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;
        let as_of = disc.base_date();

        // Base present value
        let pv = bond.value(market, as_of)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // OAS calculation requires quoted clean price
        let clean_pct = bond.pricing_overrides.quoted_clean_price
            .ok_or_else(|| PricingError::ModelFailure("OAS requires quoted clean price".to_string()))?;

        // Calculate OAS using tree pricer
        let oas_bp = TreePricer::new().calculate_oas(bond, market, as_of, clean_pct)
            .map_err(|e| PricingError::ModelFailure(e.to_string()))?;

        // Create result with OAS measure
        let mut measures = IndexMap::new();
        measures.insert("oas_bp".to_string(), oas_bp);

        let result = ValuationResult::stamped(bond.id(), as_of, pv);
        Ok(result.with_measures(measures))
    }
}
