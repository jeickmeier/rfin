//! Lightweight container aggregating all market data needed by valuations.
//!
//! `MarketContext` groups together curves (`CurveSet`), FX (`FxMatrix<P>`),
//! 2-D surfaces (`VolSurface`) and generic prices/scalars so that pricing and
//! risk components have a single handle to query required inputs.
//!
//! The container is intentionally minimal and cloning it is cheap because it
//! stores `Arc` references and small maps.

extern crate alloc;
use alloc::sync::Arc;
use hashbrown::HashMap;

use crate::money::fx::{FxMatrix, FxProvider};

use super::{
    id::CurveId,
    multicurve::CurveSet,
    primitives::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
};

/// Unified market data context
#[derive(Clone, Default)]
pub struct MarketContext<P: FxProvider> {
    /// Curves of various types (discount/forward/hazard/inflation)
    pub curves: CurveSet,
    /// Foreign-exchange matrix used for explicit FX conversions
    pub fx: Option<Arc<FxMatrix<P>>>,
    /// Volatility surfaces keyed by identifier
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,
    /// Ad-hoc prices and constants
    pub prices: HashMap<CurveId, MarketScalar>,
    /// Generic date-indexed series
    pub series: HashMap<CurveId, ScalarTimeSeries>,
}

impl<P: FxProvider> MarketContext<P> {
    /// Create an empty context.
    pub fn new() -> Self {
        Self {
            curves: CurveSet::new(),
            fx: None,
            surfaces: HashMap::new(),
            prices: HashMap::new(),
            series: HashMap::new(),
        }
    }

    /// Attach a FX matrix.
    pub fn with_fx(mut self, fx: FxMatrix<P>) -> Self {
        self.fx = Some(Arc::new(fx));
        self
    }

    /// Insert or replace a volatility surface.
    pub fn with_surface(mut self, surface: VolSurface) -> Self {
        let id = *crate::market_data::traits::TermStructure::id(&surface);
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// Insert or replace a price/scalar by id.
    pub fn with_price(mut self, id: &'static str, price: MarketScalar) -> Self {
        self.prices.insert(CurveId::new(id), price);
        self
    }

    /// Insert or replace a generic series.
    pub fn with_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = *series.id();
        self.series.insert(id, series);
        self
    }
}

impl<P: FxProvider> From<CurveSet> for MarketContext<P> {
    fn from(curves: CurveSet) -> Self {
        Self {
            curves,
            fx: None,
            surfaces: HashMap::new(),
            prices: HashMap::new(),
            series: HashMap::new(),
        }
    }
}

impl<P: FxProvider> MarketContext<P> {
    /// Convenience getters that forward to underlying containers
    pub fn vol_surface(&self, id: &'static str) -> crate::Result<Arc<VolSurface>> {
        self.surfaces
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(crate::error::InputError::NotFound.into())
    }

    pub fn market_scalar(&self, id: &'static str) -> crate::Result<&MarketScalar> {
        self.prices
            .get(&CurveId::new(id))
            .ok_or(crate::error::InputError::NotFound.into())
    }

    pub fn scalar_time_series(&self, id: &'static str) -> crate::Result<&ScalarTimeSeries> {
        self.series
            .get(&CurveId::new(id))
            .ok_or(crate::error::InputError::NotFound.into())
    }
}


