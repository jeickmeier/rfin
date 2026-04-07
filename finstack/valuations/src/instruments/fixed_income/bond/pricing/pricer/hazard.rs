//! Hazard-rate bond pricer for the pricing registry.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::bond::pricing::engine::hazard::HazardBondEngine;
use crate::instruments::fixed_income::bond::types::Bond;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;

/// Hazard-rate bond pricer using the FRP hazard engine.
///
/// This pricer uses the hazard-rate pricing engine with fractional recovery of par
/// for bonds with credit risk. Falls back to risk-free pricing if no hazard curve
/// is available in the market context.
pub(crate) struct SimpleBondHazardPricer;

impl SimpleBondHazardPricer {
    pub(crate) fn new() -> Self {
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
        let bond = instrument
            .as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::Bond, instrument.key()))?;

        let ctx = PricingErrorContext::new()
            .instrument_id(bond.id())
            .instrument_type(InstrumentType::Bond)
            .model(ModelKey::HazardRate)
            .curve_id(bond.discount_curve_id.as_str());

        let pv = HazardBondEngine::price(bond, market, as_of)
            .map_err(|e| PricingError::model_failure_with_context(e.to_string(), ctx.clone()))?;

        Ok(ValuationResult::stamped(bond.id(), as_of, pv))
    }

    fn price_raw_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<f64, PricingError> {
        let bond = instrument
            .as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::Bond, instrument.key()))?;

        let ctx = PricingErrorContext::new()
            .instrument_id(bond.id())
            .instrument_type(InstrumentType::Bond)
            .model(ModelKey::HazardRate)
            .curve_id(bond.discount_curve_id.as_str());

        HazardBondEngine::price_raw(bond, market, as_of)
            .map_err(|e| PricingError::model_failure_with_context(e.to_string(), ctx))
    }
}
