//! Lightweight container aggregating all market data needed by valuations.
//!
//! `MarketContext` groups together curves, FX (`FxMatrix`), 2-D surfaces
//! (`VolSurface`) and generic prices/scalars so that pricing and
//! risk components have a single handle to query required inputs.
//!
//! The container is intentionally minimal and cloning it is cheap because it
//! stores `Arc` references and small maps.

extern crate alloc;
use alloc::sync::Arc;
use hashbrown::HashMap;

use crate::money::fx::FxMatrix;

use super::{
    inflation::InflationCurve,
    inflation_index::InflationIndex,
    primitives::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    traits::{Discount, Forward, TermStructure},
};
use crate::types::CurveId;

/// Unified market data context
#[derive(Clone, Default)]
pub struct MarketContext {
    /// Discount curves keyed by identifier
    disc: HashMap<CurveId, Arc<dyn Discount + Send + Sync>>,
    /// Forecast curves keyed by identifier
    fwd: HashMap<CurveId, Arc<dyn Forward + Send + Sync>>,
    /// Hazard curves keyed by identifier
    hazard: HashMap<CurveId, Arc<crate::market_data::hazard_curve::HazardCurve>>,
    /// Inflation curves keyed by identifier
    inflation: HashMap<CurveId, Arc<InflationCurve>>,
    /// Inflation indices keyed by identifier
    inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,
    /// Foreign-exchange matrix used for explicit FX conversions
    pub fx: Option<Arc<FxMatrix>>,
    /// Volatility surfaces keyed by identifier
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,
    /// Ad-hoc prices and constants
    pub prices: HashMap<CurveId, MarketScalar>,
    /// Generic date-indexed series
    pub series: HashMap<CurveId, ScalarTimeSeries>,
    /// Collateral CSA code → discount curve id mapping
    collat: HashMap<&'static str, CurveId>,
}

impl MarketContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self {
            disc: HashMap::new(),
            fwd: HashMap::new(),
            hazard: HashMap::new(),
            inflation: HashMap::new(),
            inflation_indices: HashMap::new(),
            fx: None,
            surfaces: HashMap::new(),
            prices: HashMap::new(),
            series: HashMap::new(),
            collat: HashMap::new(),
        }
    }

    /// Attach a FX matrix.
    pub fn with_fx(mut self, fx: FxMatrix) -> Self {
        self.fx = Some(Arc::new(fx));
        self
    }

    /// Insert or replace a volatility surface.
    pub fn with_surface(mut self, surface: VolSurface) -> Self {
        let id = crate::market_data::traits::TermStructure::id(&surface).clone();
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// Insert or replace a price/scalar by id.
    pub fn with_price(mut self, id: impl AsRef<str>, price: MarketScalar) -> Self {
        self.prices.insert(CurveId::from(id.as_ref()), price);
        self
    }

    /// Insert or replace a generic series.
    pub fn with_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = series.id().clone();
        self.series.insert(id, series);
        self
    }

    // ------------------------------
    // Curves
    // ------------------------------
    /// Insert discount curve.
    pub fn with_discount<C: Discount + Send + Sync + 'static>(mut self, curve: C) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.disc.insert(cid, Arc::new(curve));
        self
    }

    /// Insert forecast curve.
    pub fn with_forecast<C: Forward + Send + Sync + 'static>(mut self, curve: C) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.fwd.insert(cid, Arc::new(curve));
        self
    }

    /// Insert hazard curve.
    pub fn with_hazard(mut self, curve: crate::market_data::hazard_curve::HazardCurve) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.hazard.insert(cid, Arc::new(curve));
        self
    }

    /// Insert inflation curve.
    pub fn with_inflation(mut self, curve: InflationCurve) -> Self {
        let cid = TermStructure::id(&curve).clone();
        self.inflation.insert(cid, Arc::new(curve));
        self
    }

    // Credit curve storage removed (deprecated). Use hazard curves instead.

    /// Insert inflation index.
    pub fn with_inflation_index(self, id: impl AsRef<str>, index: InflationIndex) -> Self {
        let mut this = self;
        let cid = CurveId::from(id.as_ref());
        this.inflation_indices.insert(cid, Arc::new(index));
        this
    }

    /// Map collateral CSA code to discount curve id.
    pub fn map_collateral(mut self, csa_code: &'static str, disc_id: CurveId) -> Self {
        self.collat.insert(csa_code, disc_id);
        self
    }
}

// Note: we intentionally do not provide a blanket `From<MarketContext<Q>> for MarketContext<P>`
// because it conflicts with the standard library's `impl<T> From<T> for T`.

impl MarketContext {
    /// Convenience getters that forward to underlying containers
    pub fn vol_surface(&self, id: impl AsRef<str>) -> crate::Result<Arc<VolSurface>> {
        let id_str = id.as_ref();
        self.surfaces
            .get(&CurveId::from(id_str))
            .cloned()
            .ok_or(crate::error::InputError::NotFound { id: id_str.to_string() }.into())
    }

    /// Return a reference to a market scalar (price/constant) by identifier.
    pub fn market_scalar(&self, id: impl AsRef<str>) -> crate::Result<&MarketScalar> {
        let id_str = id.as_ref();
        self.prices
            .get(&CurveId::from(id_str))
            .ok_or(crate::error::InputError::NotFound { id: id_str.to_string() }.into())
    }

    /// Return a reference to a generic date-indexed scalar time series by identifier.
    pub fn scalar_time_series(&self, id: impl AsRef<str>) -> crate::Result<&ScalarTimeSeries> {
        let id_str = id.as_ref();
        self.series
            .get(&CurveId::from(id_str))
            .ok_or(crate::error::InputError::NotFound { id: id_str.to_string() }.into())
    }

    /// Backwards compatibility alias for fetching a scalar.
    pub fn scalar(&self, id: impl AsRef<str>) -> crate::Result<&MarketScalar> {
        self.market_scalar(id)
    }

    /// Get discount curve by id.
    pub fn discount(&self, id: impl AsRef<str>) -> crate::Result<Arc<dyn Discount + Send + Sync>> {
        let id_str = id.as_ref();
        self.disc
            .get(&CurveId::from(id_str))
            .cloned()
            .ok_or(crate::error::InputError::NotFound { id: id_str.to_string() }.into())
    }

    /// Get forecast curve by id.
    pub fn forecast(&self, id: impl AsRef<str>) -> crate::Result<Arc<dyn Forward + Send + Sync>> {
        let id_str = id.as_ref();
        self.fwd
            .get(&CurveId::from(id_str))
            .cloned()
            .ok_or(crate::error::InputError::NotFound { id: id_str.to_string() }.into())
    }

    /// Get hazard curve.
    pub fn hazard(
        &self,
        id: impl AsRef<str>,
    ) -> crate::Result<Arc<crate::market_data::hazard_curve::HazardCurve>> {
        let id_str = id.as_ref();
        self.hazard
            .get(&CurveId::from(id_str))
            .cloned()
            .ok_or(crate::error::InputError::NotFound { id: id_str.to_string() }.into())
    }

    /// Get inflation curve.
    pub fn inflation(&self, id: impl AsRef<str>) -> crate::Result<Arc<InflationCurve>> {
        let id_str = id.as_ref();
        self.inflation
            .get(&CurveId::from(id_str))
            .cloned()
            .ok_or(crate::error::InputError::NotFound { id: id_str.to_string() }.into())
    }

    // Deprecated credit() getter removed. Use hazard().

    /// Get inflation index by id.
    pub fn inflation_index(&self, id: impl AsRef<str>) -> Option<Arc<InflationIndex>> {
        self.inflation_indices
            .get(&CurveId::from(id.as_ref()))
            .cloned()
    }

    /// Resolve collateral discount curve for CSA code.
    pub fn collateral(&self, csa_code: &str) -> crate::Result<Arc<dyn Discount + Send + Sync>> {
        let id = match self.collat.get(csa_code) {
            Some(cid) => cid,
            None => return Err(crate::error::InputError::NotFound { id: format!("collateral:{}", csa_code) }.into()),
        };
        self.discount(id.as_str())
    }
}
