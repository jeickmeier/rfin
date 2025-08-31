//! Extended market context for valuations with credit index data.
//!
//! Provides a wrapper around the core MarketContext that adds support for
//! complex credit market data structures needed for derivative pricing.

use crate::market_data::credit_index::CreditIndexData;
use finstack_core::market_data::context::MarketContext as CoreMarketContext;
use finstack_core::prelude::*;
use hashbrown::HashMap;
use std::sync::Arc;

/// Extended market context for valuations with credit derivatives support.
///
/// Wraps the core MarketContext and adds support for credit index data
/// required for pricing CDS tranches and other credit derivatives.
#[derive(Clone, Default)]
pub struct ValuationMarketContext {
    /// Core market context containing basic curves and data
    pub core: CoreMarketContext,
    /// Credit index data keyed by index identifier (e.g., "CDX.NA.IG.42")
    credit_indices: HashMap<String, Arc<CreditIndexData>>,
}

impl ValuationMarketContext {
    /// Create a new empty valuation market context.
    pub fn new() -> Self {
        Self {
            core: CoreMarketContext::new(),
            credit_indices: HashMap::new(),
        }
    }

    /// Create from an existing core market context.
    pub fn from_core(core: CoreMarketContext) -> Self {
        Self {
            core,
            credit_indices: HashMap::new(),
        }
    }

    /// Add credit index data for a specific index.
    pub fn with_credit_index(mut self, index_id: impl Into<String>, data: CreditIndexData) -> Self {
        self.credit_indices.insert(index_id.into(), Arc::new(data));
        self
    }

    /// Get credit index data by identifier.
    ///
    /// # Arguments
    /// * `index_id` - The credit index identifier (e.g., "CDX.NA.IG.42")
    ///
    /// # Returns
    /// Reference to the CreditIndexData if found
    ///
    /// # Errors
    /// Returns NotFound error if the index data is not available
    pub fn get_credit_index(&self, index_id: &str) -> Result<&CreditIndexData> {
        self.credit_indices
            .get(index_id)
            .map(|arc| arc.as_ref())
            .ok_or_else(|| finstack_core::error::InputError::NotFound.into())
    }

    /// Check if credit index data is available for a given index.
    pub fn has_credit_index(&self, index_id: &str) -> bool {
        self.credit_indices.contains_key(index_id)
    }

    /// Get all available credit index identifiers.
    pub fn credit_index_ids(&self) -> Vec<String> {
        self.credit_indices.keys().cloned().collect()
    }

    // Delegate common market data access to the core context

    /// Get discount curve by id (delegates to core).
    pub fn discount(
        &self,
        id: impl AsRef<str>,
    ) -> Result<Arc<dyn finstack_core::market_data::traits::Discount + Send + Sync>> {
        self.core.discount(id)
    }

    /// Get forecast curve by id (delegates to core).
    pub fn forecast(
        &self,
        id: impl AsRef<str>,
    ) -> Result<Arc<dyn finstack_core::market_data::traits::Forward + Send + Sync>> {
        self.core.forecast(id)
    }

    /// Get credit curve by id (delegates to core).
    pub fn credit(
        &self,
        id: impl AsRef<str>,
    ) -> Result<Arc<finstack_core::market_data::term_structures::credit_curve::CreditCurve>> {
        self.core.credit(id)
    }

    /// Get hazard curve by id (delegates to core).
    pub fn hazard(
        &self,
        id: impl AsRef<str>,
    ) -> Result<Arc<finstack_core::market_data::hazard_curve::HazardCurve>> {
        self.core.hazard(id)
    }

    /// Get volatility surface by id (delegates to core).
    pub fn vol_surface(
        &self,
        id: impl AsRef<str>,
    ) -> Result<Arc<finstack_core::market_data::surfaces::vol_surface::VolSurface>> {
        self.core.vol_surface(id)
    }

    /// Access the FX matrix (delegates to core).
    pub fn fx(&self) -> Option<&Arc<finstack_core::money::fx::FxMatrix>> {
        self.core.fx.as_ref()
    }
}

// Implement common builder methods by delegating to core

impl ValuationMarketContext {
    /// Add a discount curve (delegates to core).
    pub fn with_discount<
        C: finstack_core::market_data::traits::Discount + Send + Sync + 'static,
    >(
        mut self,
        curve: C,
    ) -> Self {
        self.core = self.core.with_discount(curve);
        self
    }

    /// Add a forecast curve (delegates to core).
    pub fn with_forecast<C: finstack_core::market_data::traits::Forward + Send + Sync + 'static>(
        mut self,
        curve: C,
    ) -> Self {
        self.core = self.core.with_forecast(curve);
        self
    }

    /// Add a credit curve (delegates to core).
    pub fn with_credit(
        mut self,
        curve: finstack_core::market_data::term_structures::credit_curve::CreditCurve,
    ) -> Self {
        self.core = self.core.with_credit(curve);
        self
    }

    /// Add a hazard curve (delegates to core).
    pub fn with_hazard(
        mut self,
        curve: finstack_core::market_data::hazard_curve::HazardCurve,
    ) -> Self {
        self.core = self.core.with_hazard(curve);
        self
    }

    /// Add a volatility surface (delegates to core).
    pub fn with_surface(
        mut self,
        surface: finstack_core::market_data::surfaces::vol_surface::VolSurface,
    ) -> Self {
        self.core = self.core.with_surface(surface);
        self
    }

    /// Add FX matrix (delegates to core).
    pub fn with_fx(mut self, fx: finstack_core::money::fx::FxMatrix) -> Self {
        self.core = self.core.with_fx(fx);
        self
    }
}

// Conversion methods to maintain compatibility with existing code

impl From<CoreMarketContext> for ValuationMarketContext {
    fn from(core: CoreMarketContext) -> Self {
        Self::from_core(core)
    }
}

impl From<ValuationMarketContext> for CoreMarketContext {
    fn from(valuation_ctx: ValuationMarketContext) -> Self {
        valuation_ctx.core
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::credit_index::CreditIndexData;
    use finstack_core::dates::Date;
    use finstack_core::market_data::term_structures::credit_curve::Seniority;
    use finstack_core::market_data::term_structures::{
        credit_curve::CreditCurve, BaseCorrelationCurve,
    };
    use time::Month;

    fn sample_credit_index_data() -> CreditIndexData {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        let index_curve = CreditCurve::builder("CDX.NA.IG.42")
            .issuer("CDX.NA.IG.42")
            .seniority(Seniority::Senior)
            .recovery_rate(0.40)
            .base_date(base_date)
            .spreads(vec![(1.0, 60.0), (5.0, 100.0)])
            .build()
            .unwrap();

        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![(3.0, 0.25), (10.0, 0.60), (30.0, 0.85)])
            .build()
            .unwrap();

        CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .build()
            .unwrap()
    }

    #[test]
    fn test_valuation_market_context_creation() {
        let ctx = ValuationMarketContext::new();
        assert!(ctx.credit_index_ids().is_empty());
        assert!(!ctx.has_credit_index("CDX.NA.IG.42"));
    }

    #[test]
    fn test_with_credit_index() {
        let data = sample_credit_index_data();
        let ctx = ValuationMarketContext::new().with_credit_index("CDX.NA.IG.42", data);

        assert!(ctx.has_credit_index("CDX.NA.IG.42"));
        assert_eq!(ctx.credit_index_ids(), vec!["CDX.NA.IG.42"]);

        let retrieved_data = ctx.get_credit_index("CDX.NA.IG.42").unwrap();
        assert_eq!(retrieved_data.num_constituents, 125);
        assert_eq!(retrieved_data.recovery_rate, 0.40);
    }

    #[test]
    fn test_get_nonexistent_credit_index() {
        let ctx = ValuationMarketContext::new();
        let result = ctx.get_credit_index("NONEXISTENT");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_core_context() {
        let core_ctx = CoreMarketContext::new();
        let val_ctx = ValuationMarketContext::from_core(core_ctx);
        assert!(val_ctx.credit_index_ids().is_empty());
    }

    #[test]
    fn test_conversion_between_contexts() {
        let data = sample_credit_index_data();
        let val_ctx = ValuationMarketContext::new().with_credit_index("CDX.NA.IG.42", data);

        // Convert to core context (should work)
        let core_ctx: CoreMarketContext = val_ctx.clone().into();

        // Convert back from core context
        let val_ctx2 = ValuationMarketContext::from(core_ctx);
        assert!(val_ctx2.credit_index_ids().is_empty()); // Credit indices don't survive conversion
    }
}
