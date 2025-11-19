//! CMS option Monte Carlo pricer.

use crate::instruments::cms_option::types::CmsOption;
use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingResult};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// CMS option Monte Carlo pricer.
pub struct CmsOptionMcPricer;

impl CmsOptionMcPricer {
    /// Create a new CMS option MC pricer with default config.
    pub fn new() -> Self {
        Self
    }

    /// Price a CMS option using Monte Carlo.
    fn price_internal(
        &self,
        _inst: &CmsOption,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> Result<finstack_core::money::Money> {
        // CRITICAL: Previously returned 0.0 as placeholder.
        // Explicitly failing until Hull-White model is fully implemented.
        Err(finstack_core::Error::from(PricingError::model_failure(
            "CMS Option pricing not yet implemented (Hull-White model required)".to_string(),
        )))
    }
}

impl Default for CmsOptionMcPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for CmsOptionMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CmsOption, ModelKey::MonteCarloHullWhite1F)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let cms = instrument
            .as_any()
            .downcast_ref::<CmsOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CmsOption, instrument.key())
            })?;

        let pv = self
            .price_internal(cms, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(cms.id(), as_of, pv))
    }
}

/// Present value using Monte Carlo.
pub fn npv(inst: &CmsOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = CmsOptionMcPricer::new();
    pricer.price_internal(inst, curves, as_of)
}
