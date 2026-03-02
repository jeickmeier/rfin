//! Option-adjusted spread (OAS) pricer for the pricing registry.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::bond::pricing::tree_engine::TreePricer;
use crate::instruments::fixed_income::bond::types::Bond;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext,
};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use indexmap::IndexMap;

/// Option-adjusted spread (OAS) pricer for bonds with embedded options.
///
/// This pricer calculates OAS using tree-based pricing for callable/putable bonds.
/// Requires a quoted clean price in the bond's `pricing_overrides`.
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
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        let bond = instrument
            .as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::Bond, instrument.key()))?;

        let ctx = PricingErrorContext::new()
            .instrument_id(bond.id())
            .instrument_type(InstrumentType::Bond)
            .model(ModelKey::Tree)
            .curve_id(bond.discount_curve_id.as_str());

        let pv = bond
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure_with_context(e.to_string(), ctx.clone()))?;

        let clean_pct = bond
            .pricing_overrides
            .market_quotes
            .quoted_clean_price
            .ok_or_else(|| {
                PricingError::invalid_input_with_context(
                    "OAS requires quoted clean price",
                    ctx.clone(),
                )
            })?;

        let oas_bp = TreePricer::new()
            .calculate_oas(bond, market, as_of, clean_pct)
            .map_err(|e| PricingError::model_failure_with_context(e.to_string(), ctx))?;

        let mut measures = IndexMap::new();
        measures.insert(crate::metrics::MetricId::custom("oas_bp"), oas_bp);

        let result = ValuationResult::stamped(bond.id(), as_of, pv);
        Ok(result.with_measures(measures))
    }
}
