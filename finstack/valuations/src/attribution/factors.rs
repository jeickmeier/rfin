//! Factor decomposition logic for P&L attribution.
//! Market factor manipulation for P&L attribution analysis.
//!
//! This module provides functions to selectively freeze and restore specific market
//! factors (curves, FX, volatility surfaces, scalars) while manipulating a
//! [`MarketContext`]. This is essential for attribution analysis, where we need to
//! isolate the impact of individual market moves on instrument valuations.
//!
//! # Architecture
//!
//! The module uses a **unified snapshot and restoration framework** based on bitflags
//! to eliminate code duplication. Instead of separate restore functions for each curve
//! family, we provide:
//!
//! 1. **[`CurveRestoreFlags`]** - Bitflags to specify which curve families to restore
//! 2. **[`MarketSnapshot`]** - Unified container for all curve types
//! 3. **[`MarketSnapshot::restore_market`]** - Generic restore function that works with any flag combination
//!
//! The unified API replaces older per-curve restore helpers with a single
//! `MarketSnapshot::restore_market` entry point.
//!
//! # Benefits of the Unified Approach
//!
//! - **Reduced duplication**: Eliminates ~200 lines of nearly-identical code
//! - **Composability**: Easily restore any combination of curve families with flags
//! - **Maintainability**: Single source of truth for restoration logic
//! - **Testability**: One implementation to test instead of four separate functions
//!
//! # Usage Examples
//!
//! ## Basic: Restore individual curve families
//!
//! ```rust
//! use finstack_valuations::attribution::{CurveRestoreFlags, MarketSnapshot};
//! use finstack_core::market_data::context::MarketContext;
//!
//! let market_t0 = MarketContext::new();
//! // ... populate market_t0 with curves
//!
//! let market_t1 = MarketContext::new(); // market with moved curves
//! // ... populate market_t1 with shocked curves
//!
//! // Restore t0 rates while keeping t1 credit/inflation/correlation curves
//! let rates_snapshot = MarketSnapshot::extract(&market_t0, CurveRestoreFlags::RATES);
//! let mixed_market =
//!     MarketSnapshot::restore_market(&market_t1, &rates_snapshot, CurveRestoreFlags::RATES);
//! ```
//!
//! ## Advanced: Restore arbitrary combinations with unified API
//!
//! ```rust
//! use finstack_valuations::attribution::{CurveRestoreFlags, MarketSnapshot};
//! use finstack_core::market_data::context::MarketContext;
//!
//! let market_t0 = MarketContext::new();
//! let market_t1 = MarketContext::new();
//! // ... populate both markets
//!
//! // Extract only discount and hazard curves from t0
//! let snapshot = MarketSnapshot::extract(
//!     &market_t0,
//!     CurveRestoreFlags::DISCOUNT | CurveRestoreFlags::HAZARD
//! );
//!
//! // Restore those curves to t1, preserving forward/inflation/correlation from t1
//! let mixed_market = MarketSnapshot::restore_market(
//!     &market_t1,
//!     &snapshot,
//!     CurveRestoreFlags::DISCOUNT | CurveRestoreFlags::HAZARD
//! );
//!
//! // Or restore everything except hazard curves
//! let all_but_credit = MarketSnapshot::extract(
//!     &market_t0,
//!     CurveRestoreFlags::all() & !CurveRestoreFlags::HAZARD
//! );
//! let market = MarketSnapshot::restore_market(
//!     &market_t1,
//!     &all_but_credit,
//!     CurveRestoreFlags::all() & !CurveRestoreFlags::HAZARD
//! );
//! ```
//!
//! ## P&L Attribution Workflow
//!
//! ```rust
//! use finstack_valuations::attribution::{CurveRestoreFlags, MarketSnapshot};
//! use finstack_core::market_data::context::MarketContext;
//!
//! // Start with markets at t0 and t1
//! let market_t0 = MarketContext::new();
//! let market_t1 = MarketContext::new();
//! // ... populate both markets
//!
//! // Attribute P&L to rates move
//! let rates_snapshot = MarketSnapshot::extract(&market_t0, CurveRestoreFlags::RATES);
//! let market_only_rates_moved = MarketSnapshot::restore_market(
//!     &market_t1,
//!     &rates_snapshot,
//!     CurveRestoreFlags::RATES
//! );
//! // Price with market_only_rates_moved to isolate rates P&L
//!
//! // Attribute P&L to credit move
//! let credit_snapshot = MarketSnapshot::extract(&market_t0, CurveRestoreFlags::CREDIT);
//! let market_only_credit_moved = MarketSnapshot::restore_market(
//!     &market_t1,
//!     &credit_snapshot,
//!     CurveRestoreFlags::CREDIT
//! );
//! // Price with market_only_credit_moved to isolate credit P&L
//! ```
//!
//! # Implementation Notes
//!
//! - **Preservation semantics**: Curves NOT in `restore_flags` are preserved from `current_market`
//! - **FX/surfaces/scalars**: Always preserved from `current_market` regardless of flags
//! - **Ordering**: Preserved curves inserted first, then snapshot curves (allows snapshot to override)
//! - **Empty snapshots**: Safe to pass empty snapshots; only curves present are inserted
//!
//! # See Also
//!
//! - [`crate::attribution::parallel`] - Parallel attribution using this module
//! - [`crate::attribution::waterfall`] - Waterfall attribution using this module

use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::dividends::DividendSchedule;
use finstack_core::market_data::scalars::InflationIndex;
use finstack_core::market_data::scalars::{MarketScalar, ScalarTimeSeries};
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::money::fx::FxMatrix;
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use std::sync::Arc;

/// Flags indicating which curve families to restore from snapshot vs. preserve from market.
///
/// This struct is used to control which curve types should be restored from a snapshot
/// when rebuilding a market context. Curves not marked in the flags will be preserved
/// from the original market context.
///
/// # Examples
///
/// ```
/// use finstack_valuations::attribution::CurveRestoreFlags;
///
/// // Restore only discount curves
/// let flags = CurveRestoreFlags::DISCOUNT;
///
/// // Restore both discount and forward curves (rates)
/// let rates = CurveRestoreFlags::RATES;
/// assert_eq!(rates, CurveRestoreFlags::DISCOUNT | CurveRestoreFlags::FORWARD);
///
/// // Restore everything except hazard curves
/// let all_but_credit = CurveRestoreFlags::all() & !CurveRestoreFlags::HAZARD;
///
/// // Check if discount curves should be restored
/// if rates.contains(CurveRestoreFlags::DISCOUNT) {
///     // ... restore discount curves
/// }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CurveRestoreFlags {
    /// Restore discount curves from snapshot
    pub discount: bool,
    /// Restore forward curves from snapshot
    pub forward: bool,
    /// Restore hazard curves from snapshot
    pub hazard: bool,
    /// Restore inflation curves from snapshot
    pub inflation: bool,
    /// Restore base correlation curves from snapshot
    pub correlation: bool,
}

impl CurveRestoreFlags {
    /// Restore discount curves from snapshot
    pub const DISCOUNT: Self = Self {
        discount: true,
        forward: false,
        hazard: false,
        inflation: false,
        correlation: false,
    };

    /// Restore forward curves from snapshot
    pub const FORWARD: Self = Self {
        discount: false,
        forward: true,
        hazard: false,
        inflation: false,
        correlation: false,
    };

    /// Restore hazard curves from snapshot
    pub const HAZARD: Self = Self {
        discount: false,
        forward: false,
        hazard: true,
        inflation: false,
        correlation: false,
    };

    /// Restore inflation curves from snapshot
    pub const INFLATION: Self = Self {
        discount: false,
        forward: false,
        hazard: false,
        inflation: true,
        correlation: false,
    };

    /// Restore base correlation curves from snapshot
    pub const CORRELATION: Self = Self {
        discount: false,
        forward: false,
        hazard: false,
        inflation: false,
        correlation: true,
    };

    /// Convenience combination: restore both discount and forward curves (rates family)
    pub const RATES: Self = Self {
        discount: true,
        forward: true,
        hazard: false,
        inflation: false,
        correlation: false,
    };

    /// Convenience combination: restore hazard curves (credit family)
    pub const CREDIT: Self = Self {
        discount: false,
        forward: false,
        hazard: true,
        inflation: false,
        correlation: false,
    };

    /// Returns flags with all curve types enabled.
    #[inline]
    pub const fn all() -> Self {
        Self {
            discount: true,
            forward: true,
            hazard: true,
            inflation: true,
            correlation: true,
        }
    }

    /// Returns flags with no curve types enabled.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            discount: false,
            forward: false,
            hazard: false,
            inflation: false,
            correlation: false,
        }
    }

    /// Returns true if the specified flags are all set.
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (!other.discount || self.discount)
            && (!other.forward || self.forward)
            && (!other.hazard || self.hazard)
            && (!other.inflation || self.inflation)
            && (!other.correlation || self.correlation)
    }
}

impl std::ops::BitOr for CurveRestoreFlags {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            discount: self.discount || rhs.discount,
            forward: self.forward || rhs.forward,
            hazard: self.hazard || rhs.hazard,
            inflation: self.inflation || rhs.inflation,
            correlation: self.correlation || rhs.correlation,
        }
    }
}

impl std::ops::BitAnd for CurveRestoreFlags {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            discount: self.discount && rhs.discount,
            forward: self.forward && rhs.forward,
            hazard: self.hazard && rhs.hazard,
            inflation: self.inflation && rhs.inflation,
            correlation: self.correlation && rhs.correlation,
        }
    }
}

impl std::ops::Not for CurveRestoreFlags {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self {
            discount: !self.discount,
            forward: !self.forward,
            hazard: !self.hazard,
            inflation: !self.inflation,
            correlation: !self.correlation,
        }
    }
}

/// Snapshot of volatility surfaces from a market context.
#[derive(Clone)]
pub struct VolatilitySnapshot {
    /// Volatility surfaces indexed by surface ID
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,
}

/// Snapshot of market scalars from a market context.
#[derive(Debug, Clone)]
pub struct ScalarsSnapshot {
    /// Market scalar prices indexed by ID
    pub prices: HashMap<CurveId, MarketScalar>,
    /// Time series data indexed by ID
    pub series: HashMap<CurveId, ScalarTimeSeries>,
    /// Inflation indices indexed by ID
    pub inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,
    /// Dividend schedules indexed by equity ID
    pub dividends: HashMap<CurveId, Arc<DividendSchedule>>,
}

/// Unified market snapshot that can hold any combination of curve types.
///
/// This struct provides a unified container for all curve types that can be extracted
/// from a market context. It's designed to work with `CurveRestoreFlags` to selectively
/// extract and restore different curve families.
///
/// # Examples
///
/// ```
/// use finstack_valuations::attribution::{MarketSnapshot, CurveRestoreFlags};
/// use finstack_core::market_data::context::MarketContext;
///
/// // Create a market context with some curves
/// let market = MarketContext::new();
/// // ... add curves to market
///
/// // Extract only discount and forward curves (rates family)
/// let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::RATES);
///
/// // Extract all curve types
/// let full_snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::all());
///
/// // Extract everything except credit curves
/// let no_credit = MarketSnapshot::extract(
///     &market,
///     CurveRestoreFlags::all() & !CurveRestoreFlags::HAZARD
/// );
/// ```
#[derive(Debug, Clone, Default)]
pub struct MarketSnapshot {
    /// Discount curves indexed by curve ID
    pub discount_curves: HashMap<CurveId, Arc<DiscountCurve>>,
    /// Forward curves indexed by curve ID
    pub forward_curves: HashMap<CurveId, Arc<ForwardCurve>>,
    /// Hazard curves indexed by curve ID
    pub hazard_curves: HashMap<CurveId, Arc<HazardCurve>>,
    /// Inflation curves indexed by curve ID
    pub inflation_curves: HashMap<CurveId, Arc<InflationCurve>>,
    /// Base correlation curves indexed by curve ID
    pub base_correlation_curves: HashMap<CurveId, Arc<BaseCorrelationCurve>>,
}

impl MarketSnapshot {
    /// Extract curves from a market context based on which flags are set.
    ///
    /// Only the curve types corresponding to set flags will be extracted into
    /// the snapshot. Other curve type fields will remain empty.
    ///
    /// # Arguments
    ///
    /// * `market` - Market context to extract curves from
    /// * `flags` - Bitflags indicating which curve families to extract
    ///
    /// # Returns
    ///
    /// A new `MarketSnapshot` containing only the requested curve types.
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_valuations::attribution::{MarketSnapshot, CurveRestoreFlags};
    /// use finstack_core::market_data::context::MarketContext;
    ///
    /// let market = MarketContext::new();
    /// // ... populate market with curves
    ///
    /// // Extract only discount curves
    /// let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::DISCOUNT);
    /// assert!(snapshot.forward_curves.is_empty());
    /// assert!(snapshot.hazard_curves.is_empty());
    ///
    /// // Extract both discount and forward curves
    /// let rates_snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::RATES);
    ///
    /// // Extract all curve types
    /// let full_snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::all());
    /// ```
    pub fn extract(market: &MarketContext, flags: CurveRestoreFlags) -> Self {
        let mut snapshot = Self::default();

        for curve_id in market.curve_ids() {
            // Extract discount curves if flag is set
            if flags.contains(CurveRestoreFlags::DISCOUNT) {
                if let Ok(curve) = market.get_discount(curve_id) {
                    snapshot.discount_curves.insert(curve_id.clone(), curve);
                }
            }

            // Extract forward curves if flag is set
            if flags.contains(CurveRestoreFlags::FORWARD) {
                if let Ok(curve) = market.get_forward(curve_id) {
                    snapshot.forward_curves.insert(curve_id.clone(), curve);
                }
            }

            // Extract hazard curves if flag is set
            if flags.contains(CurveRestoreFlags::HAZARD) {
                if let Ok(curve) = market.get_hazard(curve_id) {
                    snapshot.hazard_curves.insert(curve_id.clone(), curve);
                }
            }

            // Extract inflation curves if flag is set
            if flags.contains(CurveRestoreFlags::INFLATION) {
                if let Ok(curve) = market.get_inflation_curve(curve_id) {
                    snapshot.inflation_curves.insert(curve_id.clone(), curve);
                }
            }

            // Extract base correlation curves if flag is set
            if flags.contains(CurveRestoreFlags::CORRELATION) {
                if let Ok(curve) = market.get_base_correlation(curve_id) {
                    snapshot
                        .base_correlation_curves
                        .insert(curve_id.clone(), curve);
                }
            }
        }

        snapshot
    }

    /// Restore market by applying snapshot curves and preserving non-snapshot curves.
    ///
    /// This function selectively replaces curves in the market context based on the restore flags.
    /// Curves marked by `restore_flags` are taken from the snapshot, while all other curves
    /// are preserved from the current market. FX, surfaces, and scalars are always preserved
    /// from the current market.
    ///
    /// # Arguments
    ///
    /// * `current_market` - The current market context to start from
    /// * `snapshot` - Snapshot containing curves to restore
    /// * `restore_flags` - Flags indicating which curve families to restore from snapshot
    ///
    /// # Returns
    ///
    /// A new market context with the specified curves restored from snapshot and all other
    /// data preserved from the current market.
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_valuations::attribution::{MarketSnapshot, CurveRestoreFlags};
    /// use finstack_core::market_data::context::MarketContext;
    ///
    /// let current_market = MarketContext::new();
    /// // ... populate current_market with various curves
    ///
    /// // Create a snapshot with just discount curves
    /// let snapshot = MarketSnapshot::extract(&current_market, CurveRestoreFlags::DISCOUNT);
    ///
    /// // Restore only discount curves, preserve everything else
    /// let new_market = MarketSnapshot::restore_market(
    ///     &current_market,
    ///     &snapshot,
    ///     CurveRestoreFlags::DISCOUNT
    /// );
    ///
    /// // Restore all rates curves (discount + forward)
    /// let rates_snapshot = MarketSnapshot::extract(&current_market, CurveRestoreFlags::RATES);
    /// let rates_market = MarketSnapshot::restore_market(
    ///     &current_market,
    ///     &rates_snapshot,
    ///     CurveRestoreFlags::RATES
    /// );
    /// ```
    pub fn restore_market(
        current_market: &MarketContext,
        snapshot: &MarketSnapshot,
        restore_flags: CurveRestoreFlags,
    ) -> MarketContext {
        let mut new_market = MarketContext::new();

        // Determine which curves to preserve (complement of restore flags)
        // Use bitwise NOT to get all flags except the ones we're restoring
        let preserve_flags = !restore_flags & CurveRestoreFlags::all();

        // Extract preserved curves from current market
        let preserved = MarketSnapshot::extract(current_market, preserve_flags);

        // Insert preserved curves first (these are NOT being restored from snapshot)
        for curve in preserved.discount_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }
        for curve in preserved.forward_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }
        for curve in preserved.hazard_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }
        for curve in preserved.inflation_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }
        for curve in preserved.base_correlation_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }

        // Insert snapshot curves (these ARE being restored)
        // Only insert curves that were actually in the snapshot
        for curve in snapshot.discount_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }
        for curve in snapshot.forward_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }
        for curve in snapshot.hazard_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }
        for curve in snapshot.inflation_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }
        for curve in snapshot.base_correlation_curves.values() {
            new_market = new_market.insert((**curve).clone());
        }

        // Always preserve FX, surfaces, and scalars from current market
        if let Some(fx) = current_market.fx() {
            new_market = new_market.insert_fx(Arc::clone(fx));
        }
        new_market.replace_surfaces_mut(current_market.surfaces_snapshot());
        new_market = copy_scalars(current_market, new_market);

        new_market
    }
}

impl VolatilitySnapshot {
    /// Extract all volatility surfaces from a market context.
    pub fn extract(market: &MarketContext) -> Self {
        VolatilitySnapshot {
            surfaces: market.surfaces_snapshot(),
        }
    }
}

impl ScalarsSnapshot {
    /// Extract all market scalars from a market context.
    pub fn extract(market: &MarketContext) -> Self {
        ScalarsSnapshot {
            prices: market
                .prices_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            series: market
                .series_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            inflation_indices: market
                .inflation_indices_iter()
                .map(|(k, v)| (k.clone(), Arc::clone(v)))
                .collect(),
            dividends: market
                .dividends_iter()
                .map(|(k, v)| (k.clone(), Arc::clone(v)))
                .collect(),
        }
    }
}

/// Extract FX matrix from a market context.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Optional FX matrix (None if not present).
pub(crate) fn extract_fx(market: &MarketContext) -> Option<Arc<FxMatrix>> {
    market.fx().cloned()
}

fn copy_scalars(from: &MarketContext, mut to: MarketContext) -> MarketContext {
    for (id, price) in from.prices_iter() {
        to = to.insert_price(id.as_str(), price.clone());
    }
    for (_id, series) in from.series_iter() {
        to = to.insert_series(series.clone());
    }
    for (id, index) in from.inflation_indices_iter() {
        to = to.insert_inflation_index(id.as_str(), Arc::clone(index));
    }
    for (_id, schedule) in from.dividends_iter() {
        to = to.insert_dividends(Arc::clone(schedule));
    }
    to
}

/// Replace FX matrix in a market context.
///
/// # Arguments
///
/// * `market` - Market context to modify
/// * `fx` - Optional FX matrix to restore (None clears FX)
///
/// # Returns
///
/// New market context with replaced FX matrix.
pub(crate) fn restore_fx(market: &MarketContext, fx: Option<Arc<FxMatrix>>) -> MarketContext {
    match fx {
        Some(fx) => market.clone().insert_fx(fx),
        None => market.clone().clear_fx(),
    }
}

/// Replace volatility surfaces in a market context.
///
/// # Arguments
///
/// * `market` - Market context to modify
/// * `snapshot` - Snapshot of volatility surfaces to restore
///
/// # Returns
///
/// New market context with replaced volatility surfaces.
pub(crate) fn restore_volatility(
    market: &MarketContext,
    snapshot: &VolatilitySnapshot,
) -> MarketContext {
    let mut new_market = market.clone();
    new_market.replace_surfaces_mut(snapshot.surfaces.clone());
    new_market
}

/// Replace market scalars in a market context.
///
/// Rebuilds a new market preserving all curves, FX, and vol surfaces from the
/// original, but replacing ALL scalar data (prices, series, inflation indices,
/// dividends) with the snapshot values. Scalars present in `market` but absent
/// from `snapshot` are dropped to ensure clean factor isolation.
///
/// # Arguments
///
/// * `market` - Market context to modify
/// * `snapshot` - Snapshot of market scalars to restore
///
/// # Returns
///
/// New market context with replaced market scalars.
pub fn restore_scalars(market: &MarketContext, snapshot: &ScalarsSnapshot) -> MarketContext {
    let mut new_market = MarketContext::new();

    // Preserve all curves via Arc clone (cheap)
    for curve_id in market.curve_ids() {
        if let Ok(c) = market.get_discount(curve_id) {
            new_market = new_market.insert((*c).clone());
        }
        if let Ok(c) = market.get_forward(curve_id) {
            new_market = new_market.insert((*c).clone());
        }
        if let Ok(c) = market.get_hazard(curve_id) {
            new_market = new_market.insert((*c).clone());
        }
        if let Ok(c) = market.get_inflation_curve(curve_id) {
            new_market = new_market.insert((*c).clone());
        }
        if let Ok(c) = market.get_base_correlation(curve_id) {
            new_market = new_market.insert((*c).clone());
        }
    }

    // Preserve FX and surfaces
    if let Some(fx) = market.fx() {
        new_market = new_market.insert_fx(Arc::clone(fx));
    }
    new_market.replace_surfaces_mut(market.surfaces_snapshot());

    // Insert ONLY snapshot scalars (market scalars are dropped)
    for (id, scalar) in &snapshot.prices {
        new_market = new_market.insert_price(id.as_str(), scalar.clone());
    }
    for series in snapshot.series.values() {
        new_market = new_market.insert_series(series.clone());
    }
    for (id, index) in &snapshot.inflation_indices {
        new_market = new_market.insert_inflation_index(id.as_str(), Arc::clone(index));
    }
    for schedule in snapshot.dividends.values() {
        new_market = new_market.insert_dividends(Arc::clone(schedule));
    }

    new_market
}
