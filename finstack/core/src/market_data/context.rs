//! Lightweight container aggregating all market data needed by valuations.
//!
//! `MarketContext` groups together curves, FX (`FxMatrix<P>`), 2-D surfaces
//! (`VolSurface`) and generic prices/scalars so that pricing and
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
    inflation::InflationCurve,
    inflation_index::InflationIndex,
    primitives::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::credit_curve::CreditCurve,
    traits::{Discount, Forward, TermStructure},
};

/// Unified market data context
#[derive(Clone, Default)]
pub struct MarketContext<P: FxProvider> {
    /// Discount curves keyed by identifier
    disc: HashMap<CurveId, Arc<dyn Discount + Send + Sync>>,
    /// Forecast curves keyed by identifier
    fwd: HashMap<CurveId, Arc<dyn Forward + Send + Sync>>,
    /// Hazard curves keyed by identifier
    hazard: HashMap<CurveId, Arc<crate::market_data::hazard_curve::HazardCurve>>,
    /// Inflation curves keyed by identifier
    inflation: HashMap<CurveId, Arc<InflationCurve>>,
    /// Credit curves keyed by identifier
    credit: HashMap<CurveId, Arc<CreditCurve>>,
    /// Inflation indices keyed by identifier
    inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,
    /// Foreign-exchange matrix used for explicit FX conversions
    pub fx: Option<Arc<FxMatrix<P>>>,
    /// Volatility surfaces keyed by identifier
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,
    /// Ad-hoc prices and constants
    pub prices: HashMap<CurveId, MarketScalar>,
    /// Generic date-indexed series
    pub series: HashMap<CurveId, ScalarTimeSeries>,
    /// Collateral CSA code → discount curve id mapping
    collat: HashMap<&'static str, CurveId>,
}

impl<P: FxProvider> MarketContext<P> {
    /// Create an empty context.
    pub fn new() -> Self {
        Self {
            disc: HashMap::new(),
            fwd: HashMap::new(),
            hazard: HashMap::new(),
            inflation: HashMap::new(),
            credit: HashMap::new(),
            inflation_indices: HashMap::new(),
            fx: None,
            surfaces: HashMap::new(),
            prices: HashMap::new(),
            series: HashMap::new(),
            collat: HashMap::new(),
        }
    }

    /// Attach a FX matrix.
    pub fn with_fx(mut self, fx: FxMatrix<P>) -> Self {
        self.fx = Some(Arc::new(fx));
        self
    }

    /// Insert or replace a volatility surface.
    pub fn with_surface(mut self, surface: VolSurface) -> Self {
        let id = crate::market_data::traits::TermStructure::id(&surface).clone();
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// Backwards compatibility helper (former CurveSet name)
    pub fn with_vol_surface(self, surface: VolSurface) -> Self { self.with_surface(surface) }

    /// Insert or replace a price/scalar by id.
    pub fn with_price(mut self, id: impl AsRef<str>, price: MarketScalar) -> Self {
        self.prices.insert(CurveId::new(id), price);
        self
    }

    /// Backwards compatibility helper (former CurveSet name)
    pub fn with_scalar(self, id: impl AsRef<str>, scalar: MarketScalar) -> Self {
        self.with_price(id, scalar)
    }

    /// Insert or replace a generic series.
    pub fn with_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = series.id().clone();
        self.series.insert(id, series);
        self
    }

    // ------------------------------
    // Curves (formerly on CurveSet)
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

    /// Insert credit curve.
    pub fn with_credit(mut self, curve: CreditCurve) -> Self {
        let cid = curve.id.clone();
        self.credit.insert(cid, Arc::new(curve));
        self
    }

    /// Add a credit curve (mutable variant for tests)
    pub fn add_credit(&mut self, curve: CreditCurve) {
        let cid = curve.id.clone();
        self.credit.insert(cid, Arc::new(curve));
    }

    /// Insert inflation index.
    pub fn with_inflation_index(self, id: impl AsRef<str>, index: InflationIndex) -> Self {
        let mut this = self;
        let cid = CurveId::new(id);
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

impl<P: FxProvider> MarketContext<P> {
    /// Construct a `MarketContext<P>` from a backwards-compatible `CurveSet` alias.
    /// The resulting context will not carry over any FX matrix.
    pub fn from_curve_set(curve_set: crate::market_data::multicurve::CurveSet) -> Self {
        let MarketContext {
            disc,
            fwd,
            hazard,
            inflation,
            credit,
            inflation_indices,
            fx: _,
            surfaces,
            prices,
            series,
            collat,
        } = curve_set;

        Self {
            disc,
            fwd,
            hazard,
            inflation,
            credit,
            inflation_indices,
            fx: None,
            surfaces,
            prices,
            series,
            collat,
        }
    }

    /// Convenience getters that forward to underlying containers
    pub fn vol_surface(&self, id: impl AsRef<str>) -> crate::Result<Arc<VolSurface>> {
        self.surfaces
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(crate::error::InputError::NotFound.into())
    }

    /// Return a reference to a market scalar (price/constant) by identifier.
    pub fn market_scalar(&self, id: impl AsRef<str>) -> crate::Result<&MarketScalar> {
        self.prices
            .get(&CurveId::new(id))
            .ok_or(crate::error::InputError::NotFound.into())
    }

    /// Return a reference to a generic date-indexed scalar time series by identifier.
    pub fn scalar_time_series(&self, id: impl AsRef<str>) -> crate::Result<&ScalarTimeSeries> {
        self.series
            .get(&CurveId::new(id))
            .ok_or(crate::error::InputError::NotFound.into())
    }

    /// Backwards compatibility alias (former CurveSet API)
    pub fn series(&self, id: impl AsRef<str>) -> crate::Result<&ScalarTimeSeries> {
        self.scalar_time_series(id)
    }

    /// Backwards compatibility alias for fetching a scalar.
    pub fn scalar(&self, id: impl AsRef<str>) -> crate::Result<&MarketScalar> {
        self.market_scalar(id)
    }

    /// Get discount curve by id.
    pub fn discount(&self, id: impl AsRef<str>) -> crate::Result<Arc<dyn Discount + Send + Sync>> {
        self.disc
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(crate::error::InputError::NotFound.into())
    }

    /// Get forecast curve by id.
    pub fn forecast(&self, id: impl AsRef<str>) -> crate::Result<Arc<dyn Forward + Send + Sync>> {
        self.fwd
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(crate::error::InputError::NotFound.into())
    }

    /// Get hazard curve.
    pub fn hazard(&self, id: impl AsRef<str>) -> crate::Result<Arc<crate::market_data::hazard_curve::HazardCurve>> {
        self.hazard
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(crate::error::InputError::NotFound.into())
    }

    /// Get inflation curve.
    pub fn inflation(&self, id: impl AsRef<str>) -> crate::Result<Arc<InflationCurve>> {
        self.inflation
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(crate::error::InputError::NotFound.into())
    }

    /// Get credit curve by id.
    pub fn credit(&self, id: impl AsRef<str>) -> crate::Result<Arc<CreditCurve>> {
        self.credit
            .get(&CurveId::new(id))
            .cloned()
            .ok_or(crate::error::InputError::NotFound.into())
    }

    /// Get inflation index by id.
    pub fn inflation_index(&self, id: impl AsRef<str>) -> Option<Arc<InflationIndex>> {
        self.inflation_indices
            .get(&CurveId::new(id))
            .cloned()
    }

    /// Resolve collateral discount curve for CSA code.
    pub fn collateral(&self, csa_code: &str) -> crate::Result<Arc<dyn Discount + Send + Sync>> {
        let id = match self.collat.get(csa_code) {
            Some(cid) => cid,
            None => return Err(crate::error::InputError::NotFound.into()),
        };
        self.discount(id.as_str())
    }
}

// Intentionally omit `From<CurveSet>` to avoid overlap with `impl<T> From<T> for T>`.


