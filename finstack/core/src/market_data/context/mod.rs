//! Market data context for aggregating curves, surfaces, and FX rates.
//!
//! [`MarketContext`] is the primary container for market data used in valuations.
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
//! # Examples
//! ```rust
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
//! use finstack_core::math::interp::InterpStyle;
//! use finstack_core::types::CurveId;
//! use finstack_core::dates::Date;
//! use time::Month;
//!
//! let base_date = Date::from_calendar_date(2024, Month::January, 1).expect("Valid date");
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(base_date)
//!     .knots([(0.0, 1.0), (1.0, 0.98)])
//!     .set_interp(InterpStyle::Linear)
//!     .build()
//!     .expect("DiscountCurve builder should succeed");
//!
//! let ctx = MarketContext::new().insert_discount(curve);
//! let retrieved = ctx.get_discount("USD-OIS").expect("Discount curve should exist");
//! assert_eq!(retrieved.id(), &CurveId::from("USD-OIS"));
//! ```

mod curve_storage;
mod getters;
mod insert;
mod ops_bump;
mod ops_roll;
mod stats;

#[cfg(feature = "serde")]
mod state_serde;

pub use curve_storage::CurveStorage;
pub use stats::ContextStats;

// Re-export bump functionality at the same path as before.
pub use super::bumps::{BumpMode, BumpSpec, BumpUnits};

#[cfg(feature = "serde")]
pub use state_serde::{CreditIndexState, CurveState, MarketContextState};

use crate::collections::HashMap;
use std::sync::Arc;

use crate::money::fx::FxMatrix;
use crate::types::{CurveId, InstrumentId};

use super::{
    dividends::DividendSchedule,
    scalars::inflation_index::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::credit_index::CreditIndexData,
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
    pub(super) curves: HashMap<CurveId, CurveStorage>,

    /// Foreign-exchange matrix
    pub fx: Option<Arc<FxMatrix>>,

    /// Volatility surfaces
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,

    /// Market scalars and prices
    pub prices: HashMap<CurveId, MarketScalar>,

    /// Generic time series
    pub series: HashMap<CurveId, ScalarTimeSeries>,

    /// Inflation indices
    pub(super) inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,

    /// Credit index aggregates
    pub(super) credit_indices: HashMap<CurveId, Arc<CreditIndexData>>,

    /// Shared dividend schedules keyed by `CurveId` (e.g., "AAPL-DIVS")
    pub(super) dividends: HashMap<CurveId, Arc<DividendSchedule>>,

    /// Collateral CSA code mappings
    pub(super) collateral: HashMap<String, CurveId>,

    /// Type-erased instrument registry (optional, used by higher-level pricing layers).
    ///
    /// This enables workflows that need to look up referenced instruments (e.g. CTD bonds in
    /// futures) without the core crate depending on valuation-layer instrument types.
    pub(super) instruments: HashMap<InstrumentId, Arc<dyn std::any::Any + Send + Sync>>,

    /// Historical market scenarios for VaR calculation
    ///
    /// Stores time-series of historical market shifts used by Historical VaR
    /// and other scenario-based risk metrics. When present, enables VaR
    /// metric calculation.
    pub market_history: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl MarketContext {
    /// Create an empty market context.
    pub fn new() -> Self {
        Self::default()
    }
}
