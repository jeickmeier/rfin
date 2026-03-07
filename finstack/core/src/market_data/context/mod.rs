//! Market data context for aggregating curves, surfaces, and FX rates.
//!
//! `MarketContext` is the primary container for market data used in valuations.
//! It aggregates discount curves, forward curves, hazard curves, volatility
//! surfaces, FX rates, and market scalars into a single, thread-safe structure.
//!
//! # Design
//!
//! - **Arc-based storage**: Cheap to clone and share across threads
//! - **Type-safe retrieval**: Separate methods for each curve type
//! - **Builder pattern**: Fluent API for constructing contexts
//! - **Scenario support**: Bump curves for risk sensitivities
//!
//! # API boundaries
//!
//! - **Public surface**: `new`, `insert_*`, typed getters (`get_discount`, `surface`,
//!   `price`, `series`, etc.), scenario helper (`bump`), stats (`stats`) and
//!   serde states (`CurveState`, `MarketContextState`).
//! - **Internal details**: storage layout (HashMaps, caches)
//!   is not a stable API. Prefer the public methods above for all access and mutation.
//!
//! # Examples
//! ```rust
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::market_data::term_structures::DiscountCurve;
//! use finstack_core::math::interp::InterpStyle;
//! use finstack_core::types::CurveId;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let base_date = Date::from_calendar_date(2024, Month::January, 1).expect("Valid date");
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(base_date)
//!     .knots([(0.0, 1.0), (1.0, 0.98)])
//!     .interp(InterpStyle::Linear)
//!     .build()
//!     .expect("DiscountCurve builder should succeed");
//!
//! let ctx = MarketContext::new().insert(curve);
//! let retrieved = ctx.get_discount("USD-OIS").expect("Discount curve should exist");
//! assert_eq!(retrieved.id(), &CurveId::from("USD-OIS"));
//! ```

mod curve_storage;
mod getters;
mod insert;
mod ops_bump;
mod ops_roll;
mod stats;

mod state_serde;

#[doc(hidden)]
pub use curve_storage::CurveStorage;
pub use stats::ContextStats;

// Re-export bump functionality at the same path as before.
pub use super::bumps::{BumpMode, BumpSpec, BumpUnits};

pub use state_serde::{
    CreditIndexState, CurveState, MarketContextState, MARKET_CONTEXT_STATE_VERSION,
};

use crate::collections::HashMap;
use std::sync::Arc;

use crate::money::fx::FxMatrix;
use crate::types::CurveId;

use super::{
    dividends::DividendSchedule,
    scalars::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::{FxDeltaVolSurface, VolSurface},
    term_structures::CreditIndexData,
};

/// Unified market data context with enum-based storage.
///
/// The context is constructed fluently (each `insert_*` returns a new context)
/// and is cheap to clone thanks to pervasive `Arc` usage. Typical workflows
/// construct a base context at scenario initialisation and reuse it across
/// pricing engines.
#[derive(Clone, Default)]
pub struct MarketContext {
    /// All curves stored in unified enum-based map
    curves: HashMap<CurveId, CurveStorage>,

    /// Foreign-exchange matrix
    fx: Option<Arc<FxMatrix>>,

    /// Volatility surfaces
    surfaces: HashMap<CurveId, Arc<VolSurface>>,

    /// Market scalars and prices
    prices: HashMap<CurveId, MarketScalar>,

    /// Generic time series
    series: HashMap<CurveId, ScalarTimeSeries>,

    /// Inflation indices
    inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,

    /// Credit index aggregates
    credit_indices: HashMap<CurveId, Arc<CreditIndexData>>,

    /// Shared dividend schedules keyed by `CurveId` (e.g., "AAPL-DIVS")
    dividends: HashMap<CurveId, Arc<DividendSchedule>>,

    /// FX delta-quoted volatility surfaces
    fx_delta_vol_surfaces: HashMap<CurveId, Arc<FxDeltaVolSurface>>,

    /// Collateral CSA code mappings
    collateral: HashMap<String, CurveId>,
}

impl std::fmt::Debug for MarketContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarketContext")
            .field("curves", &self.curves.len())
            .field("fx", &self.fx.as_ref().map(|_| ".."))
            .field("surfaces", &self.surfaces.len())
            .field("prices", &self.prices.len())
            .field("series", &self.series.len())
            .field("inflation_indices", &self.inflation_indices.len())
            .field("credit_indices", &self.credit_indices.len())
            .field("dividends", &self.dividends.len())
            .field("fx_delta_vol_surfaces", &self.fx_delta_vol_surfaces.len())
            .field("collateral", &self.collateral.len())
            .finish()
    }
}

impl MarketContext {
    /// Create an empty market context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the FX matrix if present.
    #[inline]
    pub fn fx(&self) -> Option<&Arc<FxMatrix>> {
        self.fx.as_ref()
    }

    /// Snapshot (clone) all stored volatility surfaces.
    ///
    /// This clones the `Arc` handles (cheap) without exposing internal storage.
    #[inline]
    pub fn surfaces_snapshot(&self) -> HashMap<CurveId, Arc<VolSurface>> {
        self.surfaces.clone()
    }

    /// Snapshot (clone) all stored market scalars/prices.
    #[inline]
    pub fn prices_snapshot(&self) -> HashMap<CurveId, MarketScalar> {
        self.prices.clone()
    }

    /// Snapshot (clone) all stored scalar time series.
    #[inline]
    pub fn series_snapshot(&self) -> HashMap<CurveId, ScalarTimeSeries> {
        self.series.clone()
    }
}
