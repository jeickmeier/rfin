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

use hashbrown::HashMap;
use std::sync::Arc;

#[allow(unused_imports)] // Used in doc examples
use crate::currency::Currency;
use crate::dates::Date;
use crate::money::fx::FxMatrix;
use crate::types::CurveId;
use crate::Result;

use super::{
    dividends::DividendSchedule,
    scalars::inflation_index::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::vol_surface::VolSurface,
    term_structures::credit_index::CreditIndexData,
    term_structures::{
        base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve,
        forward_curve::ForwardCurve, hazard_curve::HazardCurve, inflation::InflationCurve,
    },
    traits::Discounting,
};

// Re-export bump functionality
use super::bumps::Bumpable;
pub use super::bumps::{BumpMode, BumpSpec, BumpUnits};

// -----------------------------------------------------------------------------
// Curve Storage
// -----------------------------------------------------------------------------

/// Unified storage for all curve types using an enum.
///
/// Downstream code rarely manipulates [`CurveStorage`] directly; it mostly
/// powers [`MarketContext`]'s heterogeneous map. When required, the helper
/// methods expose the inner `Arc` for each concrete curve type.
#[derive(Clone, Debug)]
pub enum CurveStorage {
    /// Discount factor curve
    Discount(Arc<DiscountCurve>),
    /// Forward rate curve
    Forward(Arc<ForwardCurve>),
    /// Credit hazard curve
    Hazard(Arc<HazardCurve>),
    /// Inflation index curve
    Inflation(Arc<InflationCurve>),
    /// Base correlation curve
    BaseCorrelation(Arc<BaseCorrelationCurve>),
}

// Extended API (moved from storage::curve_storage)
impl CurveStorage {
    /// Return the curve's unique identifier.
    pub fn id(&self) -> &CurveId {
        match self {
            Self::Discount(c) => c.id(),
            Self::Forward(c) => c.id(),
            Self::Hazard(c) => c.id(),
            Self::Inflation(c) => c.id(),
            Self::BaseCorrelation(c) => c.id(),
        }
    }

    /// Borrow the discount curve when the variant matches.
    pub fn discount(&self) -> Option<&Arc<DiscountCurve>> {
        match self {
            Self::Discount(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the forward curve when the variant matches.
    pub fn forward(&self) -> Option<&Arc<ForwardCurve>> {
        match self {
            Self::Forward(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the hazard curve when the variant matches.
    pub fn hazard(&self) -> Option<&Arc<HazardCurve>> {
        match self {
            Self::Hazard(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the inflation curve when the variant matches.
    pub fn inflation(&self) -> Option<&Arc<InflationCurve>> {
        match self {
            Self::Inflation(curve) => Some(curve),
            _ => None,
        }
    }

    /// Borrow the base correlation curve when the variant matches.
    pub fn base_correlation(&self) -> Option<&Arc<BaseCorrelationCurve>> {
        match self {
            Self::BaseCorrelation(curve) => Some(curve),
            _ => None,
        }
    }

    /// Return `true` when this storage contains a discount curve.
    pub fn is_discount(&self) -> bool {
        matches!(self, Self::Discount(_))
    }
    /// Return `true` when this storage contains a forward curve.
    pub fn is_forward(&self) -> bool {
        matches!(self, Self::Forward(_))
    }
    /// Return `true` when this storage contains a hazard curve.
    pub fn is_hazard(&self) -> bool {
        matches!(self, Self::Hazard(_))
    }
    /// Return `true` when this storage contains an inflation curve.
    pub fn is_inflation(&self) -> bool {
        matches!(self, Self::Inflation(_))
    }
    /// Return `true` when this storage contains a base correlation curve.
    pub fn is_base_correlation(&self) -> bool {
        matches!(self, Self::BaseCorrelation(_))
    }

    /// Return a human-readable curve type (useful for diagnostics/logging).
    pub fn curve_type(&self) -> &'static str {
        match self {
            Self::Discount(_) => "Discount",
            Self::Forward(_) => "Forward",
            Self::Hazard(_) => "Hazard",
            Self::Inflation(_) => "Inflation",
            Self::BaseCorrelation(_) => "BaseCorrelation",
        }
    }
}

// -----------------------------------------------------------------------------
// Serde: move CurveState and (De)Serialize impls here
// -----------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
/// Serializable state representation for any curve type.
///
/// Produced when the crate is compiled with the `serde` feature to persist
/// market data snapshots.
pub enum CurveState {
    /// Discount curve state
    Discount(crate::market_data::term_structures::discount_curve::DiscountCurve),
    /// Forward curve state
    Forward(crate::market_data::term_structures::forward_curve::ForwardCurve),
    /// Hazard curve state
    Hazard(crate::market_data::term_structures::hazard_curve::HazardCurve),
    /// Inflation curve state
    Inflation(crate::market_data::term_structures::inflation::InflationCurve),
    /// Base correlation curve state
    BaseCorrelation(crate::market_data::term_structures::base_correlation::BaseCorrelationCurve),
}

#[cfg(feature = "serde")]
impl CurveStorage {
    /// Convert to serializable state
    pub fn to_state(&self) -> crate::Result<CurveState> {
        Ok(match self {
            Self::Discount(curve) => CurveState::Discount((**curve).clone()),
            Self::Forward(curve) => CurveState::Forward((**curve).clone()),
            Self::Hazard(curve) => CurveState::Hazard((**curve).clone()),
            Self::Inflation(curve) => CurveState::Inflation((**curve).clone()),
            Self::BaseCorrelation(curve) => CurveState::BaseCorrelation((**curve).clone()),
        })
    }

    /// Reconstruct from serializable state
    pub fn from_state(state: CurveState) -> crate::Result<Self> {
        use std::sync::Arc;

        Ok(match state {
            CurveState::Discount(c) => Self::Discount(Arc::new(c)),
            CurveState::Forward(c) => Self::Forward(Arc::new(c)),
            CurveState::Hazard(c) => Self::Hazard(Arc::new(c)),
            CurveState::Inflation(c) => Self::Inflation(Arc::new(c)),
            CurveState::BaseCorrelation(c) => Self::BaseCorrelation(Arc::new(c)),
        })
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CurveStorage {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_state()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CurveStorage {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let state = CurveState::deserialize(deserializer)?;
        Self::from_state(state).map_err(serde::de::Error::custom)
    }
}

// -----------------------------------------------------------------------------
// Credit Index State (for serialization of CreditIndexData)
// -----------------------------------------------------------------------------

/// Serializable state for credit index data.
///
/// Instead of serializing Arc<Curve> directly, we store curve IDs that
/// reference curves present in the MarketContextState.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreditIndexState {
    /// Unique identifier for this credit index
    pub id: String,
    /// Number of constituents
    pub num_constituents: u16,
    /// Recovery rate
    pub recovery_rate: f64,
    /// ID of the index hazard curve (must exist in context curves)
    pub index_credit_curve_id: String,
    /// ID of the base correlation curve (must exist in context curves)
    pub base_correlation_curve_id: String,
    /// Optional map of issuer ID → hazard curve ID
    pub issuer_credit_curve_ids: Option<std::collections::HashMap<String, String>>,
}

// -----------------------------------------------------------------------------
// Market Context State (complete snapshot)
// -----------------------------------------------------------------------------

/// Complete serializable state of a MarketContext.
///
/// Provides a stable, versioned snapshot of all market data that can be
/// persisted to JSON and reconstructed deterministically.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketContextState {
    /// All curves (discount, forward, hazard, inflation, base correlation)
    pub curves: Vec<CurveState>,
    /// Volatility surfaces
    pub surfaces: Vec<crate::market_data::surfaces::vol_surface::VolSurface>,
    /// Market scalars and prices
    pub prices: std::collections::BTreeMap<String, crate::market_data::scalars::MarketScalar>,
    /// Generic time series
    pub series: Vec<crate::market_data::scalars::ScalarTimeSeries>,
    /// Inflation indices
    pub inflation_indices: Vec<crate::market_data::scalars::InflationIndex>,
    /// Credit index aggregates (references curves by ID)
    pub credit_indices: Vec<CreditIndexState>,
    /// Collateral CSA mappings
    pub collateral: std::collections::BTreeMap<String, String>,
}

#[cfg(feature = "serde")]
impl From<&MarketContext> for MarketContextState {
    fn from(ctx: &MarketContext) -> Self {
        // Convert all curves
        let curves: Vec<CurveState> = ctx
            .curves
            .values()
            .filter_map(|storage| storage.to_state().ok())
            .collect();

        // Convert all surfaces
        let surfaces: Vec<_> = ctx.surfaces.values().map(|surf| (**surf).clone()).collect();

        // Convert prices (CurveId → String)
        let prices: std::collections::BTreeMap<String, _> = ctx
            .prices
            .iter()
            .map(|(id, scalar)| (id.to_string(), scalar.clone()))
            .collect();

        // Convert series
        let series: Vec<_> = ctx.series.values().cloned().collect();

        // Convert inflation indices
        let inflation_indices: Vec<_> = ctx
            .inflation_indices
            .values()
            .map(|idx| (**idx).clone())
            .collect();

        // Convert credit indices (extract IDs from Arc references)
        let credit_indices: Vec<CreditIndexState> = ctx
            .credit_indices
            .iter()
            .map(|(id, data)| {
                let issuer_ids = data.issuer_credit_curves.as_ref().map(|map| {
                    map.iter()
                        .map(|(issuer, curve)| (issuer.clone(), curve.id().to_string()))
                        .collect()
                });

                CreditIndexState {
                    id: id.to_string(),
                    num_constituents: data.num_constituents,
                    recovery_rate: data.recovery_rate,
                    index_credit_curve_id: data.index_credit_curve.id().to_string(),
                    base_correlation_curve_id: data.base_correlation_curve.id().to_string(),
                    issuer_credit_curve_ids: issuer_ids,
                }
            })
            .collect();

        // Convert collateral mappings
        let collateral: std::collections::BTreeMap<String, String> = ctx
            .collateral
            .iter()
            .map(|(csa, curve_id)| (csa.clone(), curve_id.to_string()))
            .collect();

        MarketContextState {
            curves,
            surfaces,
            prices,
            series,
            inflation_indices,
            credit_indices,
            collateral,
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<MarketContextState> for MarketContext {
    type Error = crate::Error;

    fn try_from(state: MarketContextState) -> crate::Result<Self> {
        let mut ctx = MarketContext::new();

        // Reconstruct all curves
        for curve_state in state.curves {
            let storage = CurveStorage::from_state(curve_state)?;
            ctx.curves.insert(storage.id().clone(), storage);
        }

        // Reconstruct all surfaces
        for surface in state.surfaces {
            ctx.surfaces.insert(surface.id().clone(), Arc::new(surface));
        }

        // Reconstruct prices
        for (id_str, scalar) in state.prices {
            ctx.prices.insert(CurveId::from(id_str), scalar);
        }

        // Reconstruct series
        for series in state.series {
            ctx.series.insert(series.id().clone(), series);
        }

        // Reconstruct inflation indices
        for idx in state.inflation_indices {
            let id = CurveId::from(idx.id.clone());
            ctx.inflation_indices.insert(id, Arc::new(idx));
        }

        // Reconstruct credit indices (resolve curve references)
        for credit_state in state.credit_indices {
            // Resolve hazard curve
            let index_curve = ctx.get_hazard(&credit_state.index_credit_curve_id)?;

            // Resolve base correlation curve
            let base_corr = ctx.get_base_correlation(&credit_state.base_correlation_curve_id)?;

            // Resolve issuer curves if present
            let issuer_curves = if let Some(issuer_ids) = credit_state.issuer_credit_curve_ids {
                let mut map = std::collections::HashMap::new();
                for (issuer, curve_id) in issuer_ids {
                    let curve = ctx.get_hazard(&curve_id)?;
                    map.insert(issuer, curve);
                }
                Some(map)
            } else {
                None
            };

            let data = super::term_structures::credit_index::CreditIndexData {
                num_constituents: credit_state.num_constituents,
                recovery_rate: credit_state.recovery_rate,
                index_credit_curve: index_curve,
                base_correlation_curve: base_corr,
                issuer_credit_curves: issuer_curves,
            };

            ctx.credit_indices
                .insert(CurveId::from(credit_state.id), Arc::new(data));
        }

        // Reconstruct collateral mappings
        for (csa, curve_id_str) in state.collateral {
            ctx.collateral.insert(csa, CurveId::from(curve_id_str));
        }

        Ok(ctx)
    }
}

// -----------------------------------------------------------------------------
// Market Context
// -----------------------------------------------------------------------------

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

    /// Historical market scenarios for VaR calculation
    ///
    /// Stores time-series of historical market shifts used by Historical VaR
    /// and other scenario-based risk metrics. When present, enables VaR
    /// metric calculation.
    pub market_history: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl MarketContext {
    /// Create an empty market context.
    ///
    pub fn new() -> Self {
        Self::default()
    }

    // -----------------------------------------------------------------------------
    // Insert methods - builder pattern
    // -----------------------------------------------------------------------------

    /// Insert a discount curve.
    ///
    /// # Parameters
    /// - `curve`: fully built [`DiscountCurve`]
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// use finstack_core::math::interp::InterpStyle;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (1.0, 0.99)])
    ///     .build()
    ///     .expect("... builder should succeed");
    /// let ctx = MarketContext::new().insert_discount(curve);
    /// assert!(ctx.stats().total_curves > 0);
    /// ```
    pub fn insert_discount(mut self, curve: DiscountCurve) -> Self {
        let id = curve.id().to_owned();
        self.curves
            .insert(id, CurveStorage::Discount(Arc::new(curve)));
        self
    }

    /// Insert a discount curve provided as an [`Arc`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_discount_arc(mut self, curve: Arc<DiscountCurve>) -> Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::Discount(curve));
        self
    }

    /// In-place insert of a discount curve provided as an `Arc`.
    pub fn insert_discount_mut(&mut self, curve: Arc<DiscountCurve>) -> &mut Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::Discount(curve));
        self
    }

    /// Insert a forward curve.
    ///
    /// # Parameters
    /// - `curve`: fully built [`ForwardCurve`]
    pub fn insert_forward(mut self, curve: ForwardCurve) -> Self {
        let id = curve.id().to_owned();
        self.curves
            .insert(id, CurveStorage::Forward(Arc::new(curve)));
        self
    }

    /// Insert a forward curve provided as an [`Arc`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_forward_arc(mut self, curve: Arc<ForwardCurve>) -> Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::Forward(curve));
        self
    }

    /// In-place insert of a forward curve.
    pub fn insert_forward_mut(&mut self, curve: Arc<ForwardCurve>) -> &mut Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::Forward(curve));
        self
    }

    /// Insert a hazard curve.
    ///
    /// # Parameters
    /// - `curve`: fully built [`HazardCurve`]
    pub fn insert_hazard(mut self, curve: HazardCurve) -> Self {
        let id = curve.id().to_owned();
        self.curves
            .insert(id, CurveStorage::Hazard(Arc::new(curve)));
        self
    }

    /// Insert a hazard curve provided as an [`Arc`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_hazard_arc(mut self, curve: Arc<HazardCurve>) -> Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::Hazard(curve));
        self
    }

    /// In-place insert of a hazard curve.
    pub fn insert_hazard_mut(&mut self, curve: Arc<HazardCurve>) -> &mut Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::Hazard(curve));
        self
    }

    /// Insert an inflation curve.
    ///
    /// # Parameters
    /// - `curve`: fully built [`InflationCurve`]
    pub fn insert_inflation(mut self, curve: InflationCurve) -> Self {
        let id = curve.id().to_owned();
        self.curves
            .insert(id, CurveStorage::Inflation(Arc::new(curve)));
        self
    }

    /// Insert an inflation curve provided as an [`Arc`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_inflation_arc(mut self, curve: Arc<InflationCurve>) -> Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::Inflation(curve));
        self
    }

    /// In-place insert of an inflation curve.
    pub fn insert_inflation_mut(&mut self, curve: Arc<InflationCurve>) -> &mut Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::Inflation(curve));
        self
    }

    /// Insert a base correlation curve.
    ///
    /// # Parameters
    /// - `curve`: base correlation curve for structured credit pricing
    pub fn insert_base_correlation(mut self, curve: BaseCorrelationCurve) -> Self {
        let id = curve.id().to_owned();
        self.curves
            .insert(id, CurveStorage::BaseCorrelation(Arc::new(curve)));
        self
    }

    /// Insert a base correlation curve provided as an [`Arc`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_base_correlation_arc(mut self, curve: Arc<BaseCorrelationCurve>) -> Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::BaseCorrelation(curve));
        self
    }

    /// In-place insert of a base correlation curve.
    pub fn insert_base_correlation_mut(&mut self, curve: Arc<BaseCorrelationCurve>) -> &mut Self {
        let id = curve.id().to_owned();
        self.curves.insert(id, CurveStorage::BaseCorrelation(curve));
        self
    }

    /// Insert a volatility surface.
    ///
    /// # Parameters
    /// - `surface`: built [`VolSurface`]
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::surfaces::vol_surface::VolSurface;
    /// # let surface = VolSurface::builder("IR-Swaption")
    /// #     .expiries(&[1.0, 2.0])
    /// #     .strikes(&[90.0, 100.0])
    /// #     .row(&[0.2, 0.2])
    /// #     .row(&[0.2, 0.2])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// let ctx = MarketContext::new().insert_surface(surface);
    /// assert_eq!(ctx.stats().surface_count, 1);
    /// ```
    pub fn insert_surface(mut self, surface: VolSurface) -> Self {
        let id = surface.id().to_owned();
        self.surfaces.insert(id, Arc::new(surface));
        self
    }

    /// Insert a surface provided as an [`Arc`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_surface_arc(mut self, surface: Arc<VolSurface>) -> Self {
        let id = surface.id().to_owned();
        self.surfaces.insert(id, surface);
        self
    }

    /// In-place insert of a volatility surface.
    pub fn insert_surface_mut(&mut self, surface: Arc<VolSurface>) -> &mut Self {
        let id = surface.id().to_owned();
        self.surfaces.insert(id, surface);
        self
    }

    /// Insert a shared dividend schedule.
    ///
    /// # Parameters
    /// - `schedule`: a [`DividendSchedule`] built via its builder
    pub fn insert_dividends(mut self, schedule: DividendSchedule) -> Self {
        let id = schedule.id.to_owned();
        self.dividends.insert(id, Arc::new(schedule));
        self
    }

    /// Insert a dividend schedule provided as an [`Arc`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_dividends_arc(mut self, schedule: Arc<DividendSchedule>) -> Self {
        let id = schedule.id.to_owned();
        self.dividends.insert(id, schedule);
        self
    }

    /// In-place insert of a dividend schedule.
    pub fn insert_dividends_arc_mut(&mut self, schedule: Arc<DividendSchedule>) -> &mut Self {
        let id = schedule.id.to_owned();
        self.dividends.insert(id, schedule);
        self
    }

    /// Insert a market scalar/price.
    ///
    /// # Parameters
    /// - `id`: identifier (string-like) stored as [`CurveId`]
    /// - `price`: scalar value to store
    pub fn insert_price(mut self, id: impl AsRef<str>, price: MarketScalar) -> Self {
        self.prices.insert(CurveId::from(id.as_ref()), price);
        self
    }

    /// In-place insert of a market scalar/price.
    pub fn insert_price_mut(&mut self, id: impl AsRef<str>, price: MarketScalar) -> &mut Self {
        self.prices.insert(CurveId::from(id.as_ref()), price);
        self
    }

    /// Insert a scalar time series.
    ///
    /// # Parameters
    /// - `series`: [`ScalarTimeSeries`] to store
    pub fn insert_series(mut self, series: ScalarTimeSeries) -> Self {
        let id = series.id().to_owned();
        self.series.insert(id, series);
        self
    }

    /// In-place insert of a scalar time series.
    pub fn insert_series_mut(&mut self, series: ScalarTimeSeries) -> &mut Self {
        let id = series.id().to_owned();
        self.series.insert(id, series);
        self
    }

    /// Insert an inflation index.
    ///
    /// # Parameters
    /// - `id`: identifier stored as [`CurveId`]
    /// - `index`: inflation index object
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let observations = vec![
    ///     (Date::from_calendar_date(2024, Month::January, 31).expect("Valid date"), 100.0),
    ///     (Date::from_calendar_date(2024, Month::February, 29).expect("Valid date"), 101.0),
    /// ];
    /// let index = InflationIndex::new("US-CPI", observations, Currency::USD)
    ///     .expect("InflationIndex creation should succeed")
    ///     .with_interpolation(InflationInterpolation::Linear);
    /// let ctx = MarketContext::new().insert_inflation_index("US-CPI", index);
    /// assert!(ctx.inflation_index("US-CPI").is_some());
    /// ```
    pub fn insert_inflation_index(mut self, id: impl AsRef<str>, index: InflationIndex) -> Self {
        self.inflation_indices
            .insert(CurveId::from(id.as_ref()), Arc::new(index));
        self
    }

    /// Insert a credit index aggregate.
    ///
    /// # Parameters
    /// - `id`: identifier stored as [`CurveId`]
    /// - `data`: [`CreditIndexData`] bundle
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
    /// use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    /// use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
    ///     /// use finstack_core::math::interp::InterpStyle;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    /// use time::Month;
    ///
    /// let hazard = Arc::new(HazardCurve::builder("CDX")
    ///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 0.01), (5.0, 0.015)])
    ///     .build()
    ///     .expect("HazardCurve builder should succeed"));
    /// let base_corr = Arc::new(BaseCorrelationCurve::builder("CDX")
    ///     .points([(3.0, 0.25), (10.0, 0.55)])
    ///     .build()
    ///     .expect("BaseCorrelationCurve builder should succeed"));
    /// let data = CreditIndexData::builder()
    ///     .num_constituents(125)
    ///     .recovery_rate(0.4)
    ///     .index_credit_curve(Arc::clone(&hazard))
    ///     .base_correlation_curve(base_corr)
    ///     .build()
    ///     .expect("CreditIndexData builder should succeed");
    /// let ctx = MarketContext::new().insert_credit_index("CDX-IG", data);
    /// assert!(ctx.credit_index("CDX-IG").is_ok());
    /// ```
    pub fn insert_credit_index(mut self, id: impl AsRef<str>, data: CreditIndexData) -> Self {
        self.credit_indices
            .insert(CurveId::from(id.as_ref()), Arc::new(data));
        self
    }

    /// In-place insert of a credit index aggregate.
    pub fn insert_credit_index_mut(
        &mut self,
        id: impl AsRef<str>,
        data: CreditIndexData,
    ) -> &mut Self {
        self.credit_indices
            .insert(CurveId::from(id.as_ref()), Arc::new(data));
        self
    }

    /// Insert an FX matrix.
    ///
    /// # Parameters
    /// - `fx`: [`FxMatrix`] instance used for currency conversions
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use std::sync::Arc;
    /// use time::Month;
    ///
    /// struct StaticFx;
    /// impl FxProvider for StaticFx {
    ///     fn rate(
    ///         &self,
    ///         _from: Currency,
    ///         _to: Currency,
    ///         _on: Date,
    ///         _policy: FxConversionPolicy,
    ///     ) -> finstack_core::Result<f64> {
    ///         Ok(1.1)
    ///     }
    /// }
    ///
    /// let fx = FxMatrix::new(Arc::new(StaticFx));
    /// let ctx = MarketContext::new().insert_fx(fx);
    /// assert!(ctx.fx.is_some());
    /// ```
    pub fn insert_fx(mut self, fx: FxMatrix) -> Self {
        self.fx = Some(Arc::new(fx));
        self
    }

    /// Insert an FX matrix provided as an [`Arc`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert_fx_arc(mut self, fx: Arc<FxMatrix>) -> Self {
        self.fx = Some(fx);
        self
    }

    /// In-place insert of FX matrix from Arc.
    pub fn insert_fx_mut(&mut self, fx: Arc<FxMatrix>) -> &mut Self {
        self.fx = Some(fx);
        self
    }

    /// Insert historical market scenarios for VaR calculation.
    ///
    /// # Parameters
    /// - `history`: Historical market scenarios (type-erased)
    pub fn insert_market_history(mut self, history: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.market_history = Some(history);
        self
    }

    /// In-place insert of historical market scenarios.
    pub fn insert_market_history_mut(
        &mut self,
        history: Arc<dyn std::any::Any + Send + Sync>,
    ) -> &mut Self {
        self.market_history = Some(history);
        self
    }

    /// Bump FX spot rate for a currency pair and return a new context.
    ///
    /// Creates a new MarketContext with an FX matrix that has the specified
    /// currency pair rate bumped by the given percentage. All other market data
    /// is cloned unchanged.
    ///
    /// # Parameters
    /// - `from`: Base currency
    /// - `to`: Quote currency
    /// - `bump_pct`: Relative bump size (e.g., 0.01 for 1% increase)
    /// - `on`: Date for rate lookup (typically as_of date from valuation context)
    ///
    /// # Returns
    /// New MarketContext with bumped FX rate
    ///
    /// # Errors
    /// Returns error if FX matrix is missing or rate lookup fails
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::money::fx::{FxMatrix, FxProvider, FxConversionPolicy};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # struct StaticFx;
    /// # impl FxProvider for StaticFx {
    /// #     fn rate(&self, _from: Currency, _to: Currency, _on: Date, _policy: FxConversionPolicy)
    /// #         -> finstack_core::Result<f64> { Ok(1.1) }
    /// # }
    /// # let fx = FxMatrix::new(Arc::new(StaticFx));
    /// # let ctx = MarketContext::new().insert_fx(fx);
    /// # let date = Date::from_calendar_date(2024, Month::January, 1).expect("Valid date");
    /// let bumped_ctx = ctx.bump_fx_spot(Currency::EUR, Currency::USD, 0.01, date)?;
    /// // EUR/USD rate is now 1.1 * 1.01 = 1.111
    /// # Ok(())
    /// # }
    /// ```
    pub fn bump_fx_spot(
        &self,
        from: Currency,
        to: Currency,
        bump_pct: f64,
        on: Date,
    ) -> Result<Self> {
        let fx_matrix = self
            .fx
            .as_ref()
            .ok_or_else(|| crate::error::InputError::NotFound {
                id: "FX matrix".to_string(),
            })?;

        // Create new FX matrix with bumped rate
        let new_fx_matrix = fx_matrix.with_bumped_rate(from, to, bump_pct, on)?;

        // Create new context with bumped FX
        let mut new_context = self.clone();
        new_context.fx = Some(Arc::new(new_fx_matrix));

        Ok(new_context)
    }

    /// Map collateral CSA code to a discount curve identifier.
    ///
    /// # Parameters
    /// - `csa_code`: CSA identifier (e.g., "USD-CSA")
    /// - `discount_id`: target discount curve [`CurveId`]
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    ///     /// use finstack_core::math::interp::InterpStyle;
    /// use finstack_core::dates::Date;
    /// use finstack_core::types::CurveId;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (1.0, 0.99)])
    ///     .build()
    ///     .expect("... builder should succeed");
    /// let ctx = MarketContext::new()
    ///     .insert_discount(curve)
    ///     .map_collateral("USD-CSA", CurveId::from("USD-OIS"));
    /// assert!(ctx.collateral("USD-CSA").is_ok());
    /// ```
    pub fn map_collateral(mut self, csa_code: impl Into<String>, discount_id: CurveId) -> Self {
        self.collateral.insert(csa_code.into(), discount_id);
        self
    }

    /// In-place map collateral to curve id.
    pub fn map_collateral_mut(
        &mut self,
        csa_code: impl Into<String>,
        discount_id: CurveId,
    ) -> &mut Self {
        self.collateral.insert(csa_code.into(), discount_id);
        self
    }

    // -----------------------------------------------------------------------------
    // Single generic typed getters for curves
    // -----------------------------------------------------------------------------

    /// Helper method to extract curve with type checking and error handling
    fn get_curve_with_type_check<T, F>(
        &self,
        id: &str,
        expected_type: &'static str,
        extractor: F,
    ) -> Result<T>
    where
        F: FnOnce(&CurveStorage) -> Option<T>,
    {
        match self.curves.get(id) {
            Some(storage) => extractor(storage).ok_or_else(|| {
                crate::error::Error::Validation(format!(
                    "Type mismatch: curve '{}' is '{}', expected '{}'",
                    id,
                    storage.curve_type(),
                    expected_type
                ))
            }),
            None => Err(crate::error::InputError::NotFound { id: id.to_string() }.into()),
        }
    }

    /// Get a discount curve by identifier.
    pub fn get_discount(&self, id: impl AsRef<str>) -> Result<Arc<DiscountCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "Discount", |storage| {
            storage.discount().map(Arc::clone)
        })
    }

    /// Get a forward curve by identifier.
    pub fn get_forward(&self, id: impl AsRef<str>) -> Result<Arc<ForwardCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "Forward", |storage| {
            storage.forward().map(Arc::clone)
        })
    }

    /// Get a hazard curve by identifier.
    pub fn get_hazard(&self, id: impl AsRef<str>) -> Result<Arc<HazardCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "Hazard", |storage| storage.hazard().map(Arc::clone))
    }

    /// Get an inflation curve by identifier.
    pub fn get_inflation(&self, id: impl AsRef<str>) -> Result<Arc<InflationCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "Inflation", |storage| {
            storage.inflation().map(Arc::clone)
        })
    }

    /// Get a base correlation curve by identifier.
    pub fn get_base_correlation(&self, id: impl AsRef<str>) -> Result<Arc<BaseCorrelationCurve>> {
        let id_str = id.as_ref();
        self.get_curve_with_type_check(id_str, "BaseCorrelation", |storage| {
            storage.base_correlation().map(Arc::clone)
        })
    }

    /// Borrow a discount curve by identifier.
    pub fn get_discount_ref(&self, id: impl AsRef<str>) -> Result<&DiscountCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Discount(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'Discount'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Borrow a forward curve by identifier.
    pub fn get_forward_ref(&self, id: impl AsRef<str>) -> Result<&ForwardCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Forward(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'Forward'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Borrow a hazard curve by identifier.
    pub fn get_hazard_ref(&self, id: impl AsRef<str>) -> Result<&HazardCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Hazard(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'Hazard'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Borrow an inflation curve by identifier.
    pub fn get_inflation_ref(&self, id: impl AsRef<str>) -> Result<&InflationCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::Inflation(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'Inflation'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Borrow a base correlation curve by identifier.
    pub fn get_base_correlation_ref(&self, id: impl AsRef<str>) -> Result<&BaseCorrelationCurve> {
        let id_str = id.as_ref();
        match self.curves.get(id_str) {
            Some(CurveStorage::BaseCorrelation(curve)) => Ok(curve.as_ref()),
            Some(storage) => Err(crate::error::Error::Validation(format!(
                "Type mismatch: curve '{}' is '{}', expected 'BaseCorrelation'",
                id_str,
                storage.curve_type()
            ))),
            None => Err(crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into()),
        }
    }

    /// Clone a volatility surface by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::surfaces::vol_surface::VolSurface;
    /// # let surface = VolSurface::builder("IR-Swaption")
    /// #     .expiries(&[1.0, 2.0])
    /// #     .strikes(&[90.0, 100.0])
    /// #     .row(&[0.2, 0.2])
    /// #     .row(&[0.2, 0.2])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_surface(surface);
    /// let surface = ctx.surface("IR-Swaption").expect("Surface should exist");
    /// assert!((surface.value(1.5, 95.0) - 0.2).abs() < 1e-12);
    /// ```
    pub fn surface(&self, id: impl AsRef<str>) -> Result<Arc<VolSurface>> {
        let id_str = id.as_ref();
        self.surfaces.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Borrow a volatility surface without cloning the `Arc`.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::surfaces::vol_surface::VolSurface;
    /// # let surface = VolSurface::builder("IR-Swaption")
    /// #     .expiries(&[1.0, 2.0])
    /// #     .strikes(&[90.0, 100.0])
    /// #     .row(&[0.2, 0.2])
    /// #     .row(&[0.2, 0.2])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_surface(surface);
    /// let surface = ctx.surface_ref("IR-Swaption").expect("Surface should exist");
    /// assert!((surface.value(1.5, 95.0) - 0.2).abs() < 1e-12);
    /// ```
    pub fn surface_ref(&self, id: impl AsRef<str>) -> Result<&VolSurface> {
        let id_str = id.as_ref();
        self.surfaces
            .get(id_str)
            .map(|arc| arc.as_ref())
            .ok_or_else(|| {
                crate::error::Error::from(crate::error::InputError::NotFound {
                    id: id_str.to_string(),
                })
            })
    }

    /// Borrow a market price/scalar by identifier.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::scalars::MarketScalar;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let ctx = MarketContext::new()
    ///     .insert_price("AAPL", MarketScalar::Price(Money::new(180.0, Currency::USD)));
    /// if let MarketScalar::Price(price) = ctx.price("AAPL").expect("Price should exist") {
    ///     assert_eq!(price.currency(), Currency::USD);
    /// }
    /// ```
    pub fn price(&self, id: impl AsRef<str>) -> Result<&MarketScalar> {
        let id_str = id.as_ref();
        self.prices.get(id_str).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Borrow a scalar time series by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::scalars::ScalarTimeSeries;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let series = ScalarTimeSeries::new(
    /// #     "VOL-TS",
    /// #     vec![
    /// #         (Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"), 0.2),
    /// #         (Date::from_calendar_date(2024, Month::February, 1).expect("Valid date"), 0.25),
    /// #     ],
    /// #     None,
    /// # ).expect("... creation should succeed");
    /// # let ctx = MarketContext::new().insert_series(series);
    /// let series = ctx.series("VOL-TS").expect("Series should exist");
    /// assert_eq!(series.id().as_str(), "VOL-TS");
    /// ```
    pub fn series(&self, id: impl AsRef<str>) -> Result<&ScalarTimeSeries> {
        let id_str = id.as_ref();
        self.series.get(id_str).ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Clone an inflation index by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let observations = vec![
    /// #     (Date::from_calendar_date(2024, Month::January, 31).expect("Valid date"), 100.0),
    /// #     (Date::from_calendar_date(2024, Month::February, 29).expect("Valid date"), 101.0),
    /// # ];
    /// # let index = InflationIndex::new("US-CPI", observations, Currency::USD)
    /// #     .expect("... creation should succeed")
    /// #     .with_interpolation(InflationInterpolation::Linear);
    /// # let ctx = MarketContext::new().insert_inflation_index("US-CPI", index);
    /// let idx = ctx.inflation_index("US-CPI").expect("Inflation index should exist");
    /// assert_eq!(idx.id, "US-CPI");
    /// ```
    pub fn inflation_index(&self, id: impl AsRef<str>) -> Option<Arc<InflationIndex>> {
        self.inflation_indices.get(id.as_ref()).cloned()
    }

    /// Borrow an inflation index without cloning the `Arc`.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::scalars::inflation_index::{InflationIndex, InflationInterpolation};
    /// # use finstack_core::currency::Currency;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let observations = vec![
    /// #     (Date::from_calendar_date(2024, Month::January, 31).expect("Valid date"), 100.0),
    /// #     (Date::from_calendar_date(2024, Month::February, 29).expect("Valid date"), 101.0),
    /// # ];
    /// # let index = InflationIndex::new("US-CPI", observations, Currency::USD)
    /// #     .expect("... creation should succeed")
    /// #     .with_interpolation(InflationInterpolation::Linear);
    /// # let ctx = MarketContext::new().insert_inflation_index("US-CPI", index);
    /// let idx = ctx.inflation_index_ref("US-CPI").expect("Inflation index should exist");
    /// assert_eq!(idx.id, "US-CPI");
    /// ```
    pub fn inflation_index_ref(&self, id: impl AsRef<str>) -> Option<&InflationIndex> {
        self.inflation_indices
            .get(id.as_ref())
            .map(|arc| arc.as_ref())
    }

    /// Clone a dividend schedule by identifier.
    pub fn dividend_schedule(&self, id: impl AsRef<str>) -> Option<Arc<DividendSchedule>> {
        self.dividends.get(id.as_ref()).cloned()
    }

    /// Borrow a dividend schedule by identifier.
    pub fn dividend_schedule_ref(&self, id: impl AsRef<str>) -> Option<&DividendSchedule> {
        self.dividends.get(id.as_ref()).map(|arc| arc.as_ref())
    }

    /// Clone a credit index aggregate by identifier.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
    /// # use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    /// # use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
    /// #     /// # use finstack_core::math::interp::InterpStyle;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # let hazard = Arc::new(HazardCurve::builder("CDX")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 0.01), (5.0, 0.015)])
    /// #     .build()
    /// #     .expect("... creation should succeed"));
    /// # let base_corr = Arc::new(BaseCorrelationCurve::builder("CDX")
    /// #     .points([(3.0, 0.25), (10.0, 0.55)])
    /// #     .build()
    /// #     .expect("... creation should succeed"));
    /// # let data = CreditIndexData::builder()
    /// #     .num_constituents(125)
    /// #     .recovery_rate(0.4)
    /// #     .index_credit_curve(Arc::clone(&hazard))
    /// #     .base_correlation_curve(base_corr)
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_credit_index("CDX-IG", data);
    /// let idx = ctx.credit_index("CDX-IG").expect("Credit index should exist");
    /// assert_eq!(idx.num_constituents, 125);
    /// ```
    pub fn credit_index(&self, id: impl AsRef<str>) -> Result<Arc<CreditIndexData>> {
        let id_str = id.as_ref();
        self.credit_indices.get(id_str).cloned().ok_or(
            crate::error::InputError::NotFound {
                id: id_str.to_string(),
            }
            .into(),
        )
    }

    /// Borrow a credit index without cloning the `Arc`.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
    /// # use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    /// # use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
    /// #     /// # use finstack_core::math::interp::InterpStyle;
    /// # use finstack_core::dates::Date;
    /// # use std::sync::Arc;
    /// # use time::Month;
    /// # let hazard = Arc::new(HazardCurve::builder("CDX")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 0.01), (5.0, 0.015)])
    /// #     .build()
    /// #     .expect("... creation should succeed"));
    /// # let base_corr = Arc::new(BaseCorrelationCurve::builder("CDX")
    /// #     .points([(3.0, 0.25), (10.0, 0.55)])
    /// #     .build()
    /// #     .expect("... creation should succeed"));
    /// # let data = CreditIndexData::builder()
    /// #     .num_constituents(125)
    /// #     .recovery_rate(0.4)
    /// #     .index_credit_curve(Arc::clone(&hazard))
    /// #     .base_correlation_curve(base_corr)
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_credit_index("CDX-IG", data);
    /// let idx = ctx.credit_index_ref("CDX-IG").expect("Credit index should exist");
    /// assert_eq!(idx.recovery_rate, 0.4);
    /// ```
    pub fn credit_index_ref(&self, id: impl AsRef<str>) -> Result<&CreditIndexData> {
        let id_str = id.as_ref();
        self.credit_indices
            .get(id_str)
            .map(|arc| arc.as_ref())
            .ok_or_else(|| {
                crate::error::Error::from(crate::error::InputError::NotFound {
                    id: id_str.to_string(),
                })
            })
    }

    /// Resolve a collateral discount curve for a CSA code.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// use finstack_core::math::interp::InterpStyle;
    /// use finstack_core::dates::Date;
    /// use finstack_core::types::CurveId;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (1.0, 0.99)])
    ///     .build()
    ///     .expect("... builder should succeed");
    /// let ctx = MarketContext::new()
    ///     .insert_discount(curve)
    ///     .map_collateral("USD-CSA", CurveId::from("USD-OIS"));
    /// let discount = ctx.collateral("USD-CSA").expect("Collateral curve should exist");
    /// assert!(discount.df(0.5) <= 1.0);
    /// ```
    pub fn collateral(&self, csa_code: &str) -> Result<Arc<dyn Discounting + Send + Sync>> {
        let curve_id = self
            .collateral
            .get(csa_code)
            .ok_or(crate::error::InputError::NotFound {
                id: format!("collateral:{}", csa_code),
            })?;
        self.get_discount(curve_id.as_str())
            .map(|arc| arc as Arc<dyn Discounting + Send + Sync>)
    }

    /// Borrow the collateral discount curve without cloning the `Arc`.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::math::interp::InterpStyle;
    /// # use finstack_core::dates::Date;
    /// # use finstack_core::types::CurveId;
    /// # use time::Month;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 1.0), (1.0, 0.99)])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new()
    /// #     .insert_discount(curve)
    /// #     .map_collateral("USD-CSA", CurveId::from("USD-OIS"));
    /// let discount = ctx.collateral_ref("USD-CSA").expect("Collateral curve should exist");
    /// assert!(discount.df(0.5) <= 1.0);
    /// ```
    pub fn collateral_ref(&self, csa_code: &str) -> Result<&dyn Discounting> {
        let curve_id = self
            .collateral
            .get(csa_code)
            .ok_or(crate::error::InputError::NotFound {
                id: format!("collateral:{}", csa_code),
            })?;
        self.get_discount_ref(curve_id.as_str())
            .map(|r| r as &dyn Discounting)
    }

    // -----------------------------------------------------------------------------
    // Update methods for special cases
    // -----------------------------------------------------------------------------

    /// Update only the base correlation curve for a credit index.
    ///
    /// Handy for calibration loops that tweak base correlation while leaving
    /// other index data intact. Returns `false` if the index identifier cannot
    /// be found.
    pub fn update_base_correlation_curve(
        &mut self,
        id: impl AsRef<str>,
        new_curve: Arc<BaseCorrelationCurve>,
    ) -> bool {
        let cid = CurveId::from(id.as_ref());

        // Get the existing index data
        let Some(existing_index) = self.credit_indices.get(&cid) else {
            return false;
        };

        // Create a new index with the updated correlation curve
        let updated_index = CreditIndexData {
            num_constituents: existing_index.num_constituents,
            recovery_rate: existing_index.recovery_rate,
            index_credit_curve: Arc::clone(&existing_index.index_credit_curve),
            base_correlation_curve: new_curve,
            issuer_credit_curves: existing_index.issuer_credit_curves.clone(),
        };

        // Update the context
        self.credit_indices.insert(cid, Arc::new(updated_index));
        true
    }

    // -----------------------------------------------------------------------------
    // Introspection and statistics
    // -----------------------------------------------------------------------------

    /// Get curve storage by ID (for generic access)
    pub fn curve(&self, id: impl AsRef<str>) -> Option<&CurveStorage> {
        self.curves.get(id.as_ref())
    }

    /// Get all curve IDs
    pub fn curve_ids(&self) -> impl Iterator<Item = &CurveId> {
        self.curves.keys()
    }

    /// Iterate over curves matching a specific type name.
    ///
    /// # Parameters
    /// - `curve_type`: string as returned by [`CurveStorage::curve_type`]
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::math::interp::InterpStyle;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 1.0), (1.0, 0.99)])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_discount(curve);
    /// let mut iter = ctx.curves_of_type("Discount");
    /// assert!(iter.next().is_some());
    /// ```
    pub fn curves_of_type<'a>(
        &'a self,
        curve_type: &'a str,
    ) -> impl Iterator<Item = (&'a CurveId, &'a CurveStorage)> + 'a {
        self.curves
            .iter()
            .filter(move |(_, storage)| storage.curve_type() == curve_type)
    }

    /// Count curves grouped by type string.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::math::interp::InterpStyle;
    /// # use finstack_core::dates::Date;
    /// # use time::Month;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 1.0), (1.0, 0.99)])
    /// #     .build()
    /// #     .expect("... builder should succeed");
    /// # let ctx = MarketContext::new().insert_discount(curve);
    /// let counts = ctx.count_by_type();
    /// assert_eq!(counts.get("Discount"), Some(&1));
    /// ```
    pub fn count_by_type(&self) -> HashMap<&'static str, usize> {
        let mut counts = HashMap::new();
        for storage in self.curves.values() {
            *counts.entry(storage.curve_type()).or_insert(0) += 1;
        }
        counts
    }

    /// Compute aggregate statistics about the context contents.
    ///
    /// # Examples
    /// ```rust
    /// # use finstack_core::market_data::context::MarketContext;
    /// # let stats = MarketContext::new().stats();
    /// assert_eq!(stats.total_curves, 0);
    /// ```
    pub fn stats(&self) -> ContextStats {
        ContextStats {
            curve_counts: self.count_by_type(),
            total_curves: self.curves.len(),
            has_fx: self.fx.is_some(),
            surface_count: self.surfaces.len(),
            price_count: self.prices.len(),
            series_count: self.series.len(),
            inflation_index_count: self.inflation_indices.len(),
            credit_index_count: self.credit_indices.len(),
            dividend_schedule_count: self.dividends.len(),
            collateral_mapping_count: self.collateral.len(),
        }
    }

    /// Return `true` when no market data has been inserted.
    pub fn is_empty(&self) -> bool {
        self.curves.is_empty()
            && self.fx.is_none()
            && self.surfaces.is_empty()
            && self.prices.is_empty()
            && self.series.is_empty()
            && self.inflation_indices.is_empty()
            && self.credit_indices.is_empty()
            && self.collateral.is_empty()
    }

    /// Get total number of objects
    pub fn total_objects(&self) -> usize {
        self.curves.len()
            + self.surfaces.len()
            + self.prices.len()
            + self.series.len()
            + self.inflation_indices.len()
            + self.credit_indices.len()
            + if self.fx.is_some() { 1 } else { 0 }
    }

    // -----------------------------------------------------------------------------
    // Iterators for Market Scalars (P&L Attribution Support)
    // -----------------------------------------------------------------------------

    /// Iterate over all market prices/scalars.
    ///
    /// Returns an iterator over (CurveId, MarketScalar) pairs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::market_data::scalars::MarketScalar;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let ctx = MarketContext::new()
    ///     .insert_price("AAPL", MarketScalar::Price(Money::new(180.0, Currency::USD)));
    ///
    /// for (id, scalar) in ctx.prices_iter() {
    ///     println!("{}: {:?}", id, scalar);
    /// }
    /// ```
    pub fn prices_iter(&self) -> impl Iterator<Item = (&CurveId, &MarketScalar)> {
        self.prices.iter()
    }

    /// Iterate over all time series.
    ///
    /// Returns an iterator over (CurveId, ScalarTimeSeries) pairs.
    pub fn series_iter(&self) -> impl Iterator<Item = (&CurveId, &ScalarTimeSeries)> {
        self.series.iter()
    }

    /// Iterate over all inflation indices.
    ///
    /// Returns an iterator over (CurveId, Arc<InflationIndex>) pairs.
    pub fn inflation_indices_iter(&self) -> impl Iterator<Item = (&CurveId, &Arc<InflationIndex>)> {
        self.inflation_indices.iter()
    }

    /// Iterate over all dividend schedules.
    ///
    /// Returns an iterator over (CurveId, Arc<DividendSchedule>) pairs.
    pub fn dividends_iter(&self) -> impl Iterator<Item = (&CurveId, &Arc<DividendSchedule>)> {
        self.dividends.iter()
    }

    /// Set or update a market price (mutable).
    ///
    /// # Arguments
    ///
    /// * `id` - Price identifier
    /// * `price` - Market scalar to store
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn set_price_mut(&mut self, id: CurveId, price: MarketScalar) -> &mut Self {
        self.prices.insert(id, price);
        self
    }

    /// Set or update a time series (mutable).
    ///
    /// # Arguments
    ///
    /// * `series` - Time series to store
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn set_series_mut(&mut self, series: ScalarTimeSeries) -> &mut Self {
        let id = series.id().to_owned();
        self.series.insert(id, series);
        self
    }

    /// Set or update an inflation index (mutable).
    ///
    /// # Arguments
    ///
    /// * `id` - Index identifier
    /// * `index` - Inflation index to store
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn set_inflation_index_mut(
        &mut self,
        id: impl AsRef<str>,
        index: Arc<InflationIndex>,
    ) -> &mut Self {
        self.inflation_indices
            .insert(CurveId::from(id.as_ref()), index);
        self
    }

    /// Set or update a dividend schedule (mutable).
    ///
    /// # Arguments
    ///
    /// * `schedule` - Dividend schedule to store
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn set_dividends_mut(&mut self, schedule: Arc<DividendSchedule>) -> &mut Self {
        let id = schedule.id.to_owned();
        self.dividends.insert(id, schedule);
        self
    }

    // -----------------------------------------------------------------------------
    // Scenario Analysis and Stress Testing
    // -----------------------------------------------------------------------------

    /// Apply one or more bumps to the market context in a single call.
    ///
    /// This consolidated API supports discount/forward/hazard/inflation/base-correlation
    /// curves, volatility surfaces, and market scalars.
    ///
    /// # Example
    /// ```rust
    /// # use hashbrown::HashMap;
    /// # use finstack_core::market_data::context::{MarketContext, BumpSpec};
    /// # use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    /// # use finstack_core::dates::Date;
    /// # use finstack_core::types::CurveId;
    /// # let curve = DiscountCurve::builder("USD-OIS")
    /// #     .base_date(Date::from_calendar_date(2025, time::Month::January, 1).expect("Valid date"))
    /// #     .knots([(0.0, 1.0), (5.0, 0.9)])
    /// #     .build().expect("DiscountCurve builder should succeed");
    /// # let context = MarketContext::new().insert_discount(curve);
    /// let mut bumps = HashMap::new();
    /// bumps.insert(CurveId::new("USD-OIS"), BumpSpec::parallel_bp(100.0));
    /// let bumped = context.bump(bumps).expect("Bump operation should succeed");
    /// // The bumped curve replaces the original under the same ID
    /// assert!(bumped.get_discount("USD-OIS").is_ok());
    /// ```
    pub fn bump(&self, bumps: HashMap<CurveId, BumpSpec>) -> Result<Self> {
        let mut new_context = self.clone();

        for (curve_id, bump_spec) in bumps {
            let cid = curve_id.as_str();
            let mut found = false;

            if let Ok(original) = self.get_discount_ref(cid) {
                if let Some(bumped) = original.apply_bump(bump_spec) {
                    // Replace the original curve with the bumped one under the same ID
                    new_context
                        .curves
                        .insert(curve_id.clone(), CurveStorage::Discount(Arc::new(bumped)));
                    found = true;
                }
            } else if let Ok(original) = self.get_forward_ref(cid) {
                if let Some(bumped) = original.apply_bump(bump_spec) {
                    // Replace the original curve with the bumped one under the same ID
                    new_context
                        .curves
                        .insert(curve_id.clone(), CurveStorage::Forward(Arc::new(bumped)));
                    found = true;
                }
            } else if let Ok(original) = self.get_hazard_ref(cid) {
                if let Some(bumped) = original.apply_bump(bump_spec) {
                    // Replace the original curve with the bumped one under the same ID
                    new_context
                        .curves
                        .insert(curve_id.clone(), CurveStorage::Hazard(Arc::new(bumped)));
                    found = true;
                }
            } else if let Ok(original) = self.get_inflation_ref(cid) {
                if let Some(bumped) = original.apply_bump(bump_spec) {
                    // Replace the original curve with the bumped one under the same ID
                    new_context
                        .curves
                        .insert(curve_id.clone(), CurveStorage::Inflation(Arc::new(bumped)));
                    found = true;
                }
            } else if let Ok(original) = self.get_base_correlation_ref(cid) {
                if let Some(bumped) = original.apply_bump(bump_spec) {
                    // Replace the original curve with the bumped one under the same ID
                    new_context.curves.insert(
                        curve_id.clone(),
                        CurveStorage::BaseCorrelation(Arc::new(bumped)),
                    );
                    found = true;
                }
            }

            if !found {
                return Err(crate::error::InputError::NotFound {
                    id: cid.to_string(),
                }
                .into());
            }
        }

        Ok(new_context)
    }
}

// -----------------------------------------------------------------------------
// Context Statistics
// -----------------------------------------------------------------------------

/// Statistics about the contents of a [`MarketContext`].
///
/// Obtain via [`MarketContext::stats`] to feed dashboards or diagnostics.
///
/// # Examples
/// ```rust
/// use finstack_core::market_data::context::MarketContext;
///
/// let stats = MarketContext::new().stats();
/// assert_eq!(stats.total_curves, 0);
/// assert!(!stats.has_fx);
/// ```
pub struct ContextStats {
    /// Count of curves by type
    pub curve_counts: HashMap<&'static str, usize>,
    /// Total number of curves
    pub total_curves: usize,
    /// Whether FX matrix is present
    pub has_fx: bool,
    /// Number of volatility surfaces
    pub surface_count: usize,
    /// Number of market prices/scalars
    pub price_count: usize,
    /// Number of time series
    pub series_count: usize,
    /// Number of inflation indices
    pub inflation_index_count: usize,
    /// Number of credit indices
    pub credit_index_count: usize,
    /// Number of dividend schedules
    pub dividend_schedule_count: usize,
    /// Number of collateral mappings
    pub collateral_mapping_count: usize,
}

impl core::fmt::Display for ContextStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "MarketContext Statistics:")?;
        writeln!(f, "  Total curves: {}", self.total_curves)?;
        for (curve_type, count) in &self.curve_counts {
            writeln!(f, "    {}: {}", curve_type, count)?;
        }
        writeln!(f, "  Surfaces: {}", self.surface_count)?;
        writeln!(f, "  Prices: {}", self.price_count)?;
        writeln!(f, "  Series: {}", self.series_count)?;
        writeln!(f, "  Inflation indices: {}", self.inflation_index_count)?;
        writeln!(f, "  Credit indices: {}", self.credit_index_count)?;
        writeln!(f, "  Dividend schedules: {}", self.dividend_schedule_count)?;
        writeln!(
            f,
            "  Collateral mappings: {}",
            self.collateral_mapping_count
        )?;
        writeln!(f, "  Has FX: {}", self.has_fx)?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Optional Serialize/Deserialize for MarketContext (via State)
// -----------------------------------------------------------------------------

#[cfg(feature = "serde")]
impl serde::Serialize for MarketContext {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let state: MarketContextState = self.into();
        state.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for MarketContext {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let state = MarketContextState::deserialize(deserializer)?;
        Self::try_from(state).map_err(serde::de::Error::custom)
    }
}
