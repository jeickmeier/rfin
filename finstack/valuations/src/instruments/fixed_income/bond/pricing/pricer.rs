//! Bond pricer implementations for the pricing registry.
//!
//! This module provides pricer implementations that integrate bond pricing engines
//! with the instrument pricing registry system.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::bond::pricing::hazard_engine::HazardBondEngine;
use crate::instruments::fixed_income::bond::pricing::tree_engine::TreePricer;
use crate::instruments::fixed_income::bond::types::Bond;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

use indexmap::IndexMap;

// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common_impl::GenericInstrumentPricer;

/// Bond discounting pricer using the generic implementation.
///
/// This pricer uses the standard discount curve-based pricing engine for bonds
/// without embedded options or credit adjustments.
pub type SimpleBondDiscountingPricer = GenericInstrumentPricer<Bond>;

impl Default for SimpleBondDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::Bond)
    }
}

/// Hazard-rate bond pricer using the FRP hazard engine.
///
/// This pricer uses the hazard-rate pricing engine with fractional recovery of par
/// for bonds with credit risk. Falls back to risk-free pricing if no hazard curve
/// is available in the market context.
pub struct SimpleBondHazardPricer;

impl SimpleBondHazardPricer {
    /// Create a new hazard-rate bond pricer.
    ///
    /// # Returns
    ///
    /// A new `SimpleBondHazardPricer` instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleBondHazardPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for SimpleBondHazardPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Bond, ModelKey::HazardRate)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcast to Bond
        let bond = instrument
            .as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::Bond, instrument.key()))?;

        // Build error context for better diagnostics
        let ctx = PricingErrorContext::new()
            .instrument_id(bond.id())
            .instrument_type(InstrumentType::Bond)
            .model(ModelKey::HazardRate)
            .curve_id(bond.discount_curve_id.as_str());

        let pv = HazardBondEngine::price(bond, market, as_of)
            .map_err(|e| PricingError::model_failure_ctx(e.to_string(), ctx.clone()))?;

        Ok(ValuationResult::stamped(bond.id(), as_of, pv))
    }
}

/// Option-adjusted spread (OAS) pricer for bonds with embedded options.
///
/// This pricer calculates OAS using tree-based pricing for callable/putable bonds.
/// Requires a quoted clean price in the bond's `pricing_overrides`.
pub struct SimpleBondOasPricer;

impl SimpleBondOasPricer {
    /// Create a new bond OAS pricer.
    ///
    /// # Returns
    ///
    /// A new `SimpleBondOasPricer` instance.
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
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let bond = instrument
            .as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::Bond, instrument.key()))?;

        // Build error context for better diagnostics
        let ctx = PricingErrorContext::new()
            .instrument_id(bond.id())
            .instrument_type(InstrumentType::Bond)
            .model(ModelKey::Tree)
            .curve_id(bond.discount_curve_id.as_str());

        // Use the provided as_of date for consistency
        // Base present value
        let pv = bond
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure_ctx(e.to_string(), ctx.clone()))?;

        // OAS calculation requires quoted clean price
        let clean_pct = bond.pricing_overrides.quoted_clean_price.ok_or_else(|| {
            PricingError::invalid_input_ctx("OAS requires quoted clean price", ctx.clone())
        })?;

        // Calculate OAS using tree pricer
        let oas_bp = TreePricer::new()
            .calculate_oas(bond, market, as_of, clean_pct)
            .map_err(|e| PricingError::model_failure_ctx(e.to_string(), ctx))?;

        // Create result with OAS measure
        let mut measures = IndexMap::new();
        measures.insert(crate::metrics::MetricId::custom("oas_bp"), oas_bp);

        let result = ValuationResult::stamped(bond.id(), as_of, pv);
        Ok(result.with_measures(measures))
    }
}
