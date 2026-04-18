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

/// Non-optional observability for market context mutations.
///
/// Returned by operations that can invalidate credit indices or change
/// context state in non-obvious ways. Always populated regardless of
/// whether the `tracing` feature is enabled.
#[derive(Clone, Debug, Default)]
pub struct ContextMutationInfo {
    /// Credit indices that were invalidated and removed because their
    /// required curves are no longer present or changed type.
    pub invalidated_credit_indices: Vec<CurveId>,
}

impl ContextMutationInfo {
    /// Returns `true` if any credit indices were invalidated.
    #[must_use]
    pub fn has_invalidations(&self) -> bool {
        !self.invalidated_credit_indices.is_empty()
    }
}

/// Reversible in-place bump token for scratch market-context workflows.
///
/// This is intended for hot-path finite-difference calculations that need to
/// apply a small number of bumps to a reusable scratch `MarketContext` and then
/// restore the previous state without cloning the full context.
#[doc(hidden)]
#[derive(Clone)]
pub enum ContextScratchBump {
    /// A bumped curve plus the credit-index snapshot needed to restore it.
    Curve {
        /// Curve identifier that was bumped.
        id: CurveId,
        /// Pre-bump curve storage.
        previous: CurveStorage,
        /// Pre-bump credit-index snapshot.
        previous_credit_indices: HashMap<CurveId, Arc<CreditIndexData>>,
    },
    /// A bumped volatility surface.
    Surface {
        /// Surface identifier that was bumped.
        id: CurveId,
        /// Pre-bump surface.
        previous: Arc<VolSurface>,
    },
    /// A bumped scalar/price.
    Price {
        /// Scalar identifier that was bumped.
        id: CurveId,
        /// Pre-bump scalar value.
        previous: MarketScalar,
    },
}

pub use state_serde::{
    CreditIndexState, CurveState, MarketContextState, MARKET_CONTEXT_STATE_VERSION,
};

use crate::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::money::fx::FxMatrix;
use crate::types::CurveId;

use super::{
    dividends::DividendSchedule,
    hierarchy::{CompletenessReport, MarketDataHierarchy, SubtreeCoverage},
    scalars::InflationIndex,
    scalars::{MarketScalar, ScalarTimeSeries},
    surfaces::{FxDeltaVolSurface, VolCube, VolSurface},
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

    /// SABR volatility cubes (expiry x tenor x strike)
    vol_cubes: HashMap<CurveId, Arc<VolCube>>,

    /// Collateral CSA code mappings
    collateral: HashMap<String, CurveId>,

    /// Optional market data hierarchy for organizational grouping.
    hierarchy: Option<MarketDataHierarchy>,
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
            .field("vol_cubes", &self.vol_cubes.len())
            .field("collateral", &self.collateral.len())
            .field("hierarchy", &self.hierarchy.is_some())
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

    #[inline]
    pub(crate) fn inflation_index_key_for_insert(
        id: impl AsRef<str>,
        index: &InflationIndex,
    ) -> CurveId {
        let key = CurveId::from(id.as_ref());
        assert!(
            key.as_str() == index.id,
            "MarketContext::insert_inflation_index key '{}' must match InflationIndex.id '{}'",
            key.as_str(),
            index.id
        );
        key
    }

    pub(crate) fn rebind_credit_index_data(
        &self,
        data: &CreditIndexData,
    ) -> crate::Result<CreditIndexData> {
        let mut rebuilt = data.clone();

        rebuilt.index_credit_curve = self.get_hazard(rebuilt.index_credit_curve.id().as_str())?;
        rebuilt.base_correlation_curve =
            self.get_base_correlation(rebuilt.base_correlation_curve.id().as_str())?;
        if let Some(issuer_curves) = rebuilt.issuer_credit_curves.as_mut() {
            for curve in issuer_curves.values_mut() {
                *curve = self.get_hazard(curve.id().as_str())?;
            }
        }

        Ok(rebuilt)
    }

    /// Get the attached hierarchy, if any.
    pub fn hierarchy(&self) -> Option<&MarketDataHierarchy> {
        self.hierarchy.as_ref()
    }

    /// Attach a market data hierarchy.
    pub fn set_hierarchy(&mut self, h: MarketDataHierarchy) {
        self.hierarchy = Some(h);
    }

    /// Generate a completeness report comparing hierarchy declarations against
    /// all `CurveId`-keyed data stores. Returns `None` if no hierarchy is attached.
    pub fn completeness_report(&self) -> Option<CompletenessReport> {
        let hierarchy = self.hierarchy.as_ref()?;

        // Collect all CurveIds present in any store.
        let mut present: HashSet<CurveId> = HashSet::default();
        present.extend(self.curves.keys().cloned());
        present.extend(self.surfaces.keys().cloned());
        present.extend(self.prices.keys().cloned());
        present.extend(self.series.keys().cloned());
        present.extend(self.inflation_indices.keys().cloned());
        present.extend(self.credit_indices.keys().cloned());
        present.extend(self.dividends.keys().cloned());
        present.extend(self.fx_delta_vol_surfaces.keys().cloned());
        present.extend(self.vol_cubes.keys().cloned());

        // Find missing: declared in hierarchy but absent from all stores.
        let declared = hierarchy.all_curve_ids();
        let declared_set: HashSet<CurveId> = declared.iter().cloned().collect();

        let mut missing = Vec::new();
        for id in &declared {
            if !present.contains(id) {
                let path = hierarchy.path_for_curve(id).unwrap_or_default();
                debug_assert!(!path.is_empty(), "CurveId {id:?} found by all_curve_ids but not path_for_curve — tree inconsistency");
                missing.push((path, id.clone()));
            }
        }
        missing.sort_unstable_by(|a, b| a.1.cmp(&b.1));

        // Find unclassified: present in stores but not in hierarchy.
        let mut unclassified: Vec<CurveId> = present
            .iter()
            .filter(|id| !declared_set.contains(*id))
            .cloned()
            .collect();
        unclassified.sort_unstable();

        // Coverage per root subtree.
        let mut coverage = Vec::new();
        for (name, root) in hierarchy.roots() {
            let subtree_ids = root.all_curve_ids();
            let total_expected = subtree_ids.len();
            let total_present = subtree_ids
                .iter()
                .filter(|id| present.contains(*id))
                .count();
            let percent = if total_expected == 0 {
                100.0
            } else {
                (total_present as f64 / total_expected as f64) * 100.0
            };
            coverage.push(SubtreeCoverage {
                path: vec![name.clone()],
                total_expected,
                total_present,
                percent,
            });
        }

        Some(CompletenessReport {
            missing,
            unclassified,
            coverage,
        })
    }

    /// Returns `true` if any credit index references the given curve ID
    /// as its hazard curve, base correlation curve, or issuer curve.
    fn curve_affects_credit_indices(&self, curve_id: &CurveId) -> bool {
        let id_str = curve_id.as_str();
        self.credit_indices.values().any(|data| {
            data.index_credit_curve.id().as_str() == id_str
                || data.base_correlation_curve.id().as_str() == id_str
                || data
                    .issuer_credit_curves
                    .as_ref()
                    .is_some_and(|curves| curves.values().any(|c| c.id().as_str() == id_str))
        })
    }

    /// Rebind all credit indices to current curves.
    ///
    /// Returns the IDs of any credit indices that were invalidated (removed)
    /// because their required curves are no longer present or changed type.
    /// Callers can inspect this list to detect unexpected state transitions.
    pub(crate) fn rebind_all_credit_indices(&mut self) -> Vec<CurveId> {
        let mut rebuilt = HashMap::default();
        rebuilt.reserve(self.credit_indices.len());
        let mut invalidated = Vec::new();

        for (id, data) in &self.credit_indices {
            match self.rebind_credit_index_data(data) {
                Ok(index) => {
                    rebuilt.insert(id.clone(), Arc::new(index));
                }
                Err(_err) => {
                    invalidated.push(id.clone());
                    tracing::warn!(
                        credit_index_id = id.as_str(),
                        error = %_err,
                        "dropping credit index after failed curve rebinding"
                    );
                }
            }
        }

        self.credit_indices = rebuilt;
        invalidated
    }
}

#[cfg(test)]
mod tests {
    use super::MarketContext;

    #[test]
    fn market_context_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MarketContext>();
    }
}
