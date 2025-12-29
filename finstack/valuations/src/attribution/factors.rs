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
//! use finstack_valuations::attribution::factors::{CurveRestoreFlags, MarketSnapshot};
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
//! use finstack_valuations::attribution::factors::{CurveRestoreFlags, MarketSnapshot};
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
//! use finstack_valuations::attribution::factors::{CurveRestoreFlags, MarketSnapshot};
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
//! # Trait-Based Extraction (Recommended)
//!
//! The module also provides a trait-based extraction system via [`MarketExtractable`]
//! that offers better type safety and composability:
//!
//! ```rust
//! use finstack_valuations::attribution::factors::{
//!     MarketExtractable, RatesCurvesSnapshot, CreditCurvesSnapshot
//! };
//! use finstack_core::market_data::context::MarketContext;
//!
//! let market = MarketContext::new();
//! // ... populate market
//!
//! // Type-safe extraction with trait methods
//! let rates = RatesCurvesSnapshot::extract(&market);
//! let credit = CreditCurvesSnapshot::extract(&market);
//!
//! // Or use the generic helper with type inference
//! use finstack_valuations::attribution::factors::extract;
//! let rates: RatesCurvesSnapshot = extract(&market);
//! ```
//!
//! Legacy `extract_*_curves()` helpers have been removed in favor of this trait-based
//! approach, which provides better type inference and reduces the module's public API
//! surface.
//!
//! # See Also
//!
//! - [`crate::attribution::parallel`] - Parallel attribution using this module
//! - [`crate::attribution::waterfall`] - Waterfall attribution using this module

use finstack_core::HashMap;
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
/// use finstack_valuations::attribution::factors::CurveRestoreFlags;
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
    #[allow(dead_code)]
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

/// Snapshot of all discount and forward curves from a market context.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct RatesCurvesSnapshot {
    /// Discount curves indexed by curve ID
    pub discount_curves: HashMap<CurveId, Arc<DiscountCurve>>,
    /// Forward curves indexed by curve ID
    pub forward_curves: HashMap<CurveId, Arc<ForwardCurve>>,
}

/// Snapshot of all credit hazard curves from a market context.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CreditCurvesSnapshot {
    /// Hazard curves indexed by curve ID
    pub hazard_curves: HashMap<CurveId, Arc<HazardCurve>>,
}

/// Snapshot of all inflation curves from a market context.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct InflationCurvesSnapshot {
    /// Inflation curves indexed by curve ID
    pub inflation_curves: HashMap<CurveId, Arc<InflationCurve>>,
}

/// Snapshot of all base correlation curves from a market context.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CorrelationsSnapshot {
    /// Base correlation curves indexed by curve ID
    pub base_correlation_curves: HashMap<CurveId, Arc<BaseCorrelationCurve>>,
}

/// Snapshot of volatility surfaces from a market context.
#[derive(Clone)]
pub struct VolatilitySnapshot {
    /// Volatility surfaces indexed by surface ID
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,
}

/// Snapshot of market scalars from a market context.
#[derive(Clone, Debug)]
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
/// use finstack_valuations::attribution::factors::{MarketSnapshot, CurveRestoreFlags};
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
#[derive(Clone, Debug, Default)]
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
    /// use finstack_valuations::attribution::factors::{MarketSnapshot, CurveRestoreFlags};
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
                if let Ok(curve) = market.get_inflation(curve_id) {
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
    /// use finstack_valuations::attribution::factors::{MarketSnapshot, CurveRestoreFlags};
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
            new_market = new_market.insert_discount((**curve).clone());
        }
        for curve in preserved.forward_curves.values() {
            new_market = new_market.insert_forward((**curve).clone());
        }
        for curve in preserved.hazard_curves.values() {
            new_market = new_market.insert_hazard((**curve).clone());
        }
        for curve in preserved.inflation_curves.values() {
            new_market = new_market.insert_inflation((**curve).clone());
        }
        for curve in preserved.base_correlation_curves.values() {
            new_market = new_market.insert_base_correlation((**curve).clone());
        }

        // Insert snapshot curves (these ARE being restored)
        // Only insert curves that were actually in the snapshot
        for curve in snapshot.discount_curves.values() {
            new_market = new_market.insert_discount((**curve).clone());
        }
        for curve in snapshot.forward_curves.values() {
            new_market = new_market.insert_forward((**curve).clone());
        }
        for curve in snapshot.hazard_curves.values() {
            new_market = new_market.insert_hazard((**curve).clone());
        }
        for curve in snapshot.inflation_curves.values() {
            new_market = new_market.insert_inflation((**curve).clone());
        }
        for curve in snapshot.base_correlation_curves.values() {
            new_market = new_market.insert_base_correlation((**curve).clone());
        }

        // Always preserve FX, surfaces, and scalars from current market
        if let Some(fx) = &current_market.fx {
            new_market.fx = Some(Arc::clone(fx));
        }
        new_market.surfaces = current_market.surfaces.clone();
        copy_scalars(current_market, &mut new_market);

        new_market
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Trait-Based Market Data Extraction
// ═══════════════════════════════════════════════════════════════════════════════

/// Trait for types that can be extracted from a [`MarketContext`].
///
/// This trait provides a unified interface for extracting different types of market
/// data snapshots from a market context.
pub trait MarketExtractable: Sized {
    /// Extract this snapshot type from a market context.
    fn extract(market: &MarketContext) -> Self;
}

/// Generic helper function to extract a snapshot from a market context.
///
/// Uses type inference to determine which snapshot type to extract.
#[allow(dead_code)]
pub fn extract<T: MarketExtractable>(market: &MarketContext) -> T {
    T::extract(market)
}

// Implement MarketExtractable for all snapshot types
impl MarketExtractable for RatesCurvesSnapshot {
    fn extract(market: &MarketContext) -> Self {
        let mut discount_curves = HashMap::default();
        let mut forward_curves = HashMap::default();

        // Use public API to iterate through curves
        for curve_id in market.curve_ids() {
            // Try to get as discount curve
            if let Ok(discount) = market.get_discount(curve_id) {
                discount_curves.insert(curve_id.clone(), discount);
            }
            // Try to get as forward curve
            else if let Ok(forward) = market.get_forward(curve_id) {
                forward_curves.insert(curve_id.clone(), forward);
            }
        }

        RatesCurvesSnapshot {
            discount_curves,
            forward_curves,
        }
    }
}

impl MarketExtractable for CreditCurvesSnapshot {
    fn extract(market: &MarketContext) -> Self {
        let mut hazard_curves = HashMap::default();

        for curve_id in market.curve_ids() {
            if let Ok(hazard) = market.get_hazard(curve_id) {
                hazard_curves.insert(curve_id.clone(), hazard);
            }
        }

        CreditCurvesSnapshot { hazard_curves }
    }
}

impl MarketExtractable for InflationCurvesSnapshot {
    fn extract(market: &MarketContext) -> Self {
        let mut inflation_curves = HashMap::default();

        for curve_id in market.curve_ids() {
            if let Ok(inflation) = market.get_inflation(curve_id) {
                inflation_curves.insert(curve_id.clone(), inflation);
            }
        }

        InflationCurvesSnapshot { inflation_curves }
    }
}

impl MarketExtractable for CorrelationsSnapshot {
    fn extract(market: &MarketContext) -> Self {
        let mut base_correlation_curves = HashMap::default();

        for curve_id in market.curve_ids() {
            if let Ok(base_corr) = market.get_base_correlation(curve_id) {
                base_correlation_curves.insert(curve_id.clone(), base_corr);
            }
        }

        CorrelationsSnapshot {
            base_correlation_curves,
        }
    }
}

impl MarketExtractable for VolatilitySnapshot {
    fn extract(market: &MarketContext) -> Self {
        VolatilitySnapshot {
            surfaces: market.surfaces.clone(),
        }
    }
}

impl MarketExtractable for ScalarsSnapshot {
    fn extract(market: &MarketContext) -> Self {
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
pub fn extract_fx(market: &MarketContext) -> Option<Arc<FxMatrix>> {
    market.fx.clone()
}

fn copy_scalars(from: &MarketContext, to: &mut MarketContext) {
    for (id, price) in from.prices_iter() {
        to.set_price_mut(id.clone(), price.clone());
    }
    for (_id, series) in from.series_iter() {
        to.set_series_mut(series.clone());
    }
    for (id, index) in from.inflation_indices_iter() {
        to.set_inflation_index_mut(id.as_str(), Arc::clone(index));
    }
    for (_id, schedule) in from.dividends_iter() {
        to.set_dividends_mut(Arc::clone(schedule));
    }
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
pub fn restore_fx(market: &MarketContext, fx: Option<Arc<FxMatrix>>) -> MarketContext {
    let mut new_market = market.clone();
    new_market.fx = fx;
    new_market
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
pub fn restore_volatility(market: &MarketContext, snapshot: &VolatilitySnapshot) -> MarketContext {
    let mut new_market = market.clone();
    new_market.surfaces = snapshot.surfaces.clone();
    new_market
}

/// Replace market scalars in a market context.
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
    // Create a fresh market context
    let mut new_market = MarketContext::new();

    // Copy all curves
    for curve_id in market.curve_ids() {
        if let Ok(discount) = market.get_discount(curve_id) {
            let owned = Arc::try_unwrap(discount).unwrap_or_else(|arc| arc.as_ref().clone());
            new_market = new_market.insert_discount(owned);
        } else if let Ok(forward) = market.get_forward(curve_id) {
            let owned = Arc::try_unwrap(forward).unwrap_or_else(|arc| arc.as_ref().clone());
            new_market = new_market.insert_forward(owned);
        } else if let Ok(hazard) = market.get_hazard(curve_id) {
            let owned = Arc::try_unwrap(hazard).unwrap_or_else(|arc| arc.as_ref().clone());
            new_market = new_market.insert_hazard(owned);
        } else if let Ok(inflation) = market.get_inflation(curve_id) {
            let owned = Arc::try_unwrap(inflation).unwrap_or_else(|arc| arc.as_ref().clone());
            new_market = new_market.insert_inflation(owned);
        } else if let Ok(base_corr) = market.get_base_correlation(curve_id) {
            let owned = Arc::try_unwrap(base_corr).unwrap_or_else(|arc| arc.as_ref().clone());
            new_market = new_market.insert_base_correlation(owned);
        }
    }

    // Copy FX and surfaces
    if let Some(fx) = &market.fx {
        new_market.fx = Some(Arc::clone(fx));
    }
    new_market.surfaces = market.surfaces.clone();

    // Restore scalars from snapshot (overwrites any existing)
    for (id, scalar) in &snapshot.prices {
        new_market.set_price_mut(id.clone(), scalar.clone());
    }
    for series in snapshot.series.values() {
        new_market.set_series_mut(series.clone());
    }
    for (id, index) in &snapshot.inflation_indices {
        new_market.set_inflation_index_mut(id.as_str(), Arc::clone(index));
    }
    for schedule in snapshot.dividends.values() {
        new_market.set_dividends_mut(Arc::clone(schedule));
    }

    new_market
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use finstack_core::math::interp::InterpStyle;
    use time::macros::date;

    fn create_test_discount_curve(id: &str, base_date: Date) -> DiscountCurve {
        DiscountCurve::builder(id)
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data")
    }

    #[test]
    fn test_extract_and_restore_rates_curves() {
        let base_date = date!(2025 - 01 - 15);
        let curve1 = create_test_discount_curve("USD-OIS", base_date);
        let curve2 = create_test_discount_curve("EUR-OIS", base_date);

        let market = MarketContext::new()
            .insert_discount(curve1)
            .insert_discount(curve2);

        // Extract snapshot
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::RATES);
        assert_eq!(snapshot.discount_curves.len(), 2);

        // Create empty market and restore
        let empty_market = MarketContext::new();
        let restored =
            MarketSnapshot::restore_market(&empty_market, &snapshot, CurveRestoreFlags::RATES);

        assert!(restored.get_discount("USD-OIS").is_ok());
        assert!(restored.get_discount("EUR-OIS").is_ok());
    }

    #[test]
    fn test_curve_restore_flags_individual() {
        // Test individual flags are distinct
        assert_ne!(CurveRestoreFlags::DISCOUNT, CurveRestoreFlags::FORWARD);
        assert_ne!(CurveRestoreFlags::DISCOUNT, CurveRestoreFlags::HAZARD);
        assert_ne!(CurveRestoreFlags::HAZARD, CurveRestoreFlags::INFLATION);
        assert_ne!(CurveRestoreFlags::INFLATION, CurveRestoreFlags::CORRELATION);
    }

    #[test]
    fn test_curve_restore_flags_union() {
        // Test union operations
        let discount_forward = CurveRestoreFlags::DISCOUNT | CurveRestoreFlags::FORWARD;
        assert!(discount_forward.contains(CurveRestoreFlags::DISCOUNT));
        assert!(discount_forward.contains(CurveRestoreFlags::FORWARD));
        assert!(!discount_forward.contains(CurveRestoreFlags::HAZARD));

        // Test RATES convenience constant
        assert_eq!(CurveRestoreFlags::RATES, discount_forward);
        assert!(CurveRestoreFlags::RATES.contains(CurveRestoreFlags::DISCOUNT));
        assert!(CurveRestoreFlags::RATES.contains(CurveRestoreFlags::FORWARD));
    }

    #[test]
    fn test_curve_restore_flags_intersection() {
        // Test intersection operations
        let rates = CurveRestoreFlags::RATES;
        let discount_hazard = CurveRestoreFlags::DISCOUNT | CurveRestoreFlags::HAZARD;

        let intersection = rates & discount_hazard;
        assert!(intersection.contains(CurveRestoreFlags::DISCOUNT));
        assert!(!intersection.contains(CurveRestoreFlags::FORWARD));
        assert!(!intersection.contains(CurveRestoreFlags::HAZARD));
    }

    #[test]
    fn test_curve_restore_flags_complement() {
        // Test complement operations
        let all = CurveRestoreFlags::all();
        let not_discount = all & !CurveRestoreFlags::DISCOUNT;

        assert!(!not_discount.contains(CurveRestoreFlags::DISCOUNT));
        assert!(not_discount.contains(CurveRestoreFlags::FORWARD));
        assert!(not_discount.contains(CurveRestoreFlags::HAZARD));
        assert!(not_discount.contains(CurveRestoreFlags::INFLATION));
        assert!(not_discount.contains(CurveRestoreFlags::CORRELATION));
    }

    #[test]
    fn test_curve_restore_flags_all_but_credit() {
        // Test combining complement with intersection
        let all_but_credit = CurveRestoreFlags::all() & !CurveRestoreFlags::HAZARD;

        assert!(all_but_credit.contains(CurveRestoreFlags::DISCOUNT));
        assert!(all_but_credit.contains(CurveRestoreFlags::FORWARD));
        assert!(!all_but_credit.contains(CurveRestoreFlags::HAZARD));
        assert!(all_but_credit.contains(CurveRestoreFlags::INFLATION));
        assert!(all_but_credit.contains(CurveRestoreFlags::CORRELATION));
    }

    #[test]
    fn test_curve_restore_flags_credit_constant() {
        // Test CREDIT convenience constant
        assert_eq!(CurveRestoreFlags::CREDIT, CurveRestoreFlags::HAZARD);
        assert!(CurveRestoreFlags::CREDIT.contains(CurveRestoreFlags::HAZARD));
        assert!(!CurveRestoreFlags::CREDIT.contains(CurveRestoreFlags::DISCOUNT));
    }

    #[test]
    fn test_curve_restore_flags_empty() {
        // Test empty flags
        let empty = CurveRestoreFlags::empty();
        assert!(!empty.contains(CurveRestoreFlags::DISCOUNT));
        assert!(!empty.contains(CurveRestoreFlags::FORWARD));
        assert!(!empty.contains(CurveRestoreFlags::HAZARD));
        assert!(!empty.contains(CurveRestoreFlags::INFLATION));
        assert!(!empty.contains(CurveRestoreFlags::CORRELATION));
    }

    #[test]
    fn test_curve_restore_flags_all() {
        // Test all flags
        let all = CurveRestoreFlags::all();
        assert!(all.contains(CurveRestoreFlags::DISCOUNT));
        assert!(all.contains(CurveRestoreFlags::FORWARD));
        assert!(all.contains(CurveRestoreFlags::HAZARD));
        assert!(all.contains(CurveRestoreFlags::INFLATION));
        assert!(all.contains(CurveRestoreFlags::CORRELATION));
        assert!(all.contains(CurveRestoreFlags::RATES));
        assert!(all.contains(CurveRestoreFlags::CREDIT));
    }

    #[test]
    fn test_curve_restore_flags_bitwise_combinations() {
        // Test complex bitwise combinations
        let rates_and_inflation = CurveRestoreFlags::RATES | CurveRestoreFlags::INFLATION;
        assert!(rates_and_inflation.contains(CurveRestoreFlags::DISCOUNT));
        assert!(rates_and_inflation.contains(CurveRestoreFlags::FORWARD));
        assert!(rates_and_inflation.contains(CurveRestoreFlags::INFLATION));
        assert!(!rates_and_inflation.contains(CurveRestoreFlags::HAZARD));
        assert!(!rates_and_inflation.contains(CurveRestoreFlags::CORRELATION));

        // Test subtraction using complement
        let only_discount = CurveRestoreFlags::RATES & !CurveRestoreFlags::FORWARD;
        assert!(only_discount.contains(CurveRestoreFlags::DISCOUNT));
        assert!(!only_discount.contains(CurveRestoreFlags::FORWARD));
    }

    fn create_test_forward_curve(id: &str, base_date: Date) -> ForwardCurve {
        ForwardCurve::builder(id, 0.25) // 3M tenor
            .base_date(base_date)
            .knots(vec![(0.0, 0.02), (1.0, 0.025), (5.0, 0.03)])
            .build()
            .expect("ForwardCurve builder should succeed with valid test data")
    }

    fn create_test_hazard_curve(id: &str, base_date: Date) -> HazardCurve {
        HazardCurve::builder(id)
            .base_date(base_date)
            .knots(vec![(0.0, 0.0050), (1.0, 0.0055), (5.0, 0.0060)])
            .build()
            .expect("HazardCurve builder should succeed with valid test data")
    }

    fn create_test_inflation_curve(id: &str, _base_date: Date) -> InflationCurve {
        InflationCurve::builder(id)
            .base_cpi(100.0)
            .knots(vec![(0.0, 100.0), (1.0, 102.0), (5.0, 110.0)])
            .build()
            .expect("InflationCurve builder should succeed with valid test data")
    }

    fn create_test_base_correlation_curve(id: &str, _base_date: Date) -> BaseCorrelationCurve {
        BaseCorrelationCurve::builder(id)
            .knots(vec![
                (0.03, 0.30), // 3% detach
                (0.07, 0.40), // 7% detach
                (0.10, 0.50), // 10% detach
                (0.15, 0.60), // 15% detach
                (0.30, 0.70), // 30% detach
            ])
            .build()
            .expect("BaseCorrelationCurve builder should succeed with valid test data")
    }

    #[test]
    fn test_market_snapshot_extract_single_discount() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);

        let market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve);

        // Extract only discount curves
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::DISCOUNT);

        assert_eq!(snapshot.discount_curves.len(), 1);
        assert!(snapshot.discount_curves.contains_key("USD-OIS"));
        assert!(snapshot.forward_curves.is_empty());
        assert!(snapshot.hazard_curves.is_empty());
        assert!(snapshot.inflation_curves.is_empty());
        assert!(snapshot.base_correlation_curves.is_empty());
    }

    #[test]
    fn test_market_snapshot_extract_rates_combination() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);

        let market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve);

        // Extract rates (discount + forward)
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::RATES);

        assert_eq!(snapshot.discount_curves.len(), 1);
        assert_eq!(snapshot.forward_curves.len(), 1);
        assert!(snapshot.discount_curves.contains_key("USD-OIS"));
        assert!(snapshot.forward_curves.contains_key("USD-SOFR"));
        assert!(snapshot.hazard_curves.is_empty());
        assert!(snapshot.inflation_curves.is_empty());
        assert!(snapshot.base_correlation_curves.is_empty());
    }

    #[test]
    fn test_market_snapshot_extract_all_curve_types() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);
        let inflation_curve = create_test_inflation_curve("US-CPI", base_date);
        let base_corr_curve = create_test_base_correlation_curve("CDX-IG", base_date);

        let market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve)
            .insert_inflation(inflation_curve)
            .insert_base_correlation(base_corr_curve);

        // Extract all curve types
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::all());

        assert_eq!(snapshot.discount_curves.len(), 1);
        assert_eq!(snapshot.forward_curves.len(), 1);
        assert_eq!(snapshot.hazard_curves.len(), 1);
        assert_eq!(snapshot.inflation_curves.len(), 1);
        assert_eq!(snapshot.base_correlation_curves.len(), 1);
        assert!(snapshot.discount_curves.contains_key("USD-OIS"));
        assert!(snapshot.forward_curves.contains_key("USD-SOFR"));
        assert!(snapshot.hazard_curves.contains_key("CORP-A"));
        assert!(snapshot.inflation_curves.contains_key("US-CPI"));
        assert!(snapshot.base_correlation_curves.contains_key("CDX-IG"));
    }

    #[test]
    fn test_market_snapshot_extract_empty_flags() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);

        let market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve);

        // Extract with empty flags (nothing should be extracted)
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::empty());

        assert!(snapshot.discount_curves.is_empty());
        assert!(snapshot.forward_curves.is_empty());
        assert!(snapshot.hazard_curves.is_empty());
        assert!(snapshot.inflation_curves.is_empty());
        assert!(snapshot.base_correlation_curves.is_empty());
    }

    #[test]
    fn test_market_snapshot_extract_from_empty_market() {
        let market = MarketContext::new();

        // Extract from empty market with all flags
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::all());

        assert!(snapshot.discount_curves.is_empty());
        assert!(snapshot.forward_curves.is_empty());
        assert!(snapshot.hazard_curves.is_empty());
        assert!(snapshot.inflation_curves.is_empty());
        assert!(snapshot.base_correlation_curves.is_empty());
    }

    #[test]
    fn test_market_snapshot_extract_multiple_curves_same_type() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve1 = create_test_discount_curve("USD-OIS", base_date);
        let discount_curve2 = create_test_discount_curve("EUR-OIS", base_date);
        let discount_curve3 = create_test_discount_curve("GBP-OIS", base_date);

        let market = MarketContext::new()
            .insert_discount(discount_curve1)
            .insert_discount(discount_curve2)
            .insert_discount(discount_curve3);

        // Extract only discount curves
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::DISCOUNT);

        assert_eq!(snapshot.discount_curves.len(), 3);
        assert!(snapshot.discount_curves.contains_key("USD-OIS"));
        assert!(snapshot.discount_curves.contains_key("EUR-OIS"));
        assert!(snapshot.discount_curves.contains_key("GBP-OIS"));
        assert!(snapshot.forward_curves.is_empty());
    }

    #[test]
    fn test_market_snapshot_extract_complement_flags() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);

        let market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve);

        // Extract everything except hazard curves
        let flags = CurveRestoreFlags::all() & !CurveRestoreFlags::HAZARD;
        let snapshot = MarketSnapshot::extract(&market, flags);

        assert_eq!(snapshot.discount_curves.len(), 1);
        assert_eq!(snapshot.forward_curves.len(), 1);
        assert!(snapshot.hazard_curves.is_empty());
        assert!(snapshot.discount_curves.contains_key("USD-OIS"));
        assert!(snapshot.forward_curves.contains_key("USD-SOFR"));
    }

    #[test]
    fn test_market_snapshot_default() {
        let snapshot = MarketSnapshot::default();

        assert!(snapshot.discount_curves.is_empty());
        assert!(snapshot.forward_curves.is_empty());
        assert!(snapshot.hazard_curves.is_empty());
        assert!(snapshot.inflation_curves.is_empty());
        assert!(snapshot.base_correlation_curves.is_empty());
    }

    #[test]
    fn test_restore_market_unified_discount_only() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);

        let current_market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve);

        // Create snapshot with a different discount curve
        let new_discount = create_test_discount_curve("EUR-OIS", base_date);
        let snapshot = MarketSnapshot {
            discount_curves: vec![("EUR-OIS".into(), Arc::new(new_discount))]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        // Restore only discount curves
        let restored =
            MarketSnapshot::restore_market(&current_market, &snapshot, CurveRestoreFlags::DISCOUNT);

        // Should have new discount curve from snapshot
        assert!(restored.get_discount("EUR-OIS").is_ok());
        // Original discount curve should be replaced
        assert!(restored.get_discount("USD-OIS").is_err());
        // Forward and hazard curves should be preserved
        assert!(restored.get_forward("USD-SOFR").is_ok());
        assert!(restored.get_hazard("CORP-A").is_ok());
    }

    #[test]
    fn test_restore_market_unified_rates() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);

        let current_market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve);

        // Create snapshot with new rates curves
        let new_discount = create_test_discount_curve("EUR-OIS", base_date);
        let new_forward = create_test_forward_curve("EUR-ESTR", base_date);
        let snapshot = MarketSnapshot {
            discount_curves: vec![("EUR-OIS".into(), Arc::new(new_discount))]
                .into_iter()
                .collect(),
            forward_curves: vec![("EUR-ESTR".into(), Arc::new(new_forward))]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        // Restore rates curves (discount + forward)
        let restored =
            MarketSnapshot::restore_market(&current_market, &snapshot, CurveRestoreFlags::RATES);

        // Should have new rates curves from snapshot
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_forward("EUR-ESTR").is_ok());
        // Original rates curves should be replaced
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_forward("USD-SOFR").is_err());
        // Hazard curve should be preserved
        assert!(restored.get_hazard("CORP-A").is_ok());
    }

    #[test]
    fn test_restore_market_unified_all_curves() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);
        let inflation_curve = create_test_inflation_curve("US-CPI", base_date);
        let base_corr_curve = create_test_base_correlation_curve("CDX-IG", base_date);

        let current_market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve)
            .insert_inflation(inflation_curve)
            .insert_base_correlation(base_corr_curve);

        // Create snapshot with new curves for all types
        let new_discount = create_test_discount_curve("EUR-OIS", base_date);
        let new_forward = create_test_forward_curve("EUR-ESTR", base_date);
        let new_hazard = create_test_hazard_curve("CORP-B", base_date);
        let new_inflation = create_test_inflation_curve("EU-HICP", base_date);
        let new_base_corr = create_test_base_correlation_curve("ITRAXX", base_date);

        let snapshot = MarketSnapshot {
            discount_curves: vec![("EUR-OIS".into(), Arc::new(new_discount))]
                .into_iter()
                .collect(),
            forward_curves: vec![("EUR-ESTR".into(), Arc::new(new_forward))]
                .into_iter()
                .collect(),
            hazard_curves: vec![("CORP-B".into(), Arc::new(new_hazard))]
                .into_iter()
                .collect(),
            inflation_curves: vec![("EU-HICP".into(), Arc::new(new_inflation))]
                .into_iter()
                .collect(),
            base_correlation_curves: vec![("ITRAXX".into(), Arc::new(new_base_corr))]
                .into_iter()
                .collect(),
        };

        // Restore all curve types
        let restored =
            MarketSnapshot::restore_market(&current_market, &snapshot, CurveRestoreFlags::all());

        // Should have all new curves from snapshot
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_forward("EUR-ESTR").is_ok());
        assert!(restored.get_hazard("CORP-B").is_ok());
        assert!(restored.get_inflation("EU-HICP").is_ok());
        assert!(restored.get_base_correlation("ITRAXX").is_ok());

        // Original curves should be replaced
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_forward("USD-SOFR").is_err());
        assert!(restored.get_hazard("CORP-A").is_err());
        assert!(restored.get_inflation("US-CPI").is_err());
        assert!(restored.get_base_correlation("CDX-IG").is_err());
    }

    #[test]
    fn test_restore_market_unified_preserve_non_restore_curves() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);
        let inflation_curve = create_test_inflation_curve("US-CPI", base_date);

        let current_market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve)
            .insert_inflation(inflation_curve);

        // Create snapshot with only new hazard curve
        let new_hazard = create_test_hazard_curve("CORP-B", base_date);
        let snapshot = MarketSnapshot {
            hazard_curves: vec![("CORP-B".into(), Arc::new(new_hazard))]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        // Restore only hazard curves (credit)
        let restored =
            MarketSnapshot::restore_market(&current_market, &snapshot, CurveRestoreFlags::CREDIT);

        // Should have new hazard curve from snapshot
        assert!(restored.get_hazard("CORP-B").is_ok());
        // Original hazard curve should be replaced
        assert!(restored.get_hazard("CORP-A").is_err());
        // All other curves should be preserved
        assert!(restored.get_discount("USD-OIS").is_ok());
        assert!(restored.get_forward("USD-SOFR").is_ok());
        assert!(restored.get_inflation("US-CPI").is_ok());
    }

    #[test]
    fn test_restore_market_unified_empty_snapshot() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);

        let current_market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve);

        // Create empty snapshot
        let snapshot = MarketSnapshot::default();

        // Restore with RATES flag but empty snapshot
        let restored =
            MarketSnapshot::restore_market(&current_market, &snapshot, CurveRestoreFlags::RATES);

        // No curves should exist (snapshot was empty, so all rates curves removed)
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_forward("USD-SOFR").is_err());
    }

    #[test]
    fn test_restore_market_unified_empty_current_market() {
        let base_date = date!(2025 - 01 - 15);
        let current_market = MarketContext::new();

        // Create snapshot with curves
        let new_discount = create_test_discount_curve("USD-OIS", base_date);
        let new_forward = create_test_forward_curve("USD-SOFR", base_date);
        let snapshot = MarketSnapshot {
            discount_curves: vec![("USD-OIS".into(), Arc::new(new_discount))]
                .into_iter()
                .collect(),
            forward_curves: vec![("USD-SOFR".into(), Arc::new(new_forward))]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        // Restore into empty market
        let restored =
            MarketSnapshot::restore_market(&current_market, &snapshot, CurveRestoreFlags::RATES);

        // Should have curves from snapshot
        assert!(restored.get_discount("USD-OIS").is_ok());
        assert!(restored.get_forward("USD-SOFR").is_ok());
    }

    #[test]
    fn test_restore_market_unified_complement_flags() {
        let base_date = date!(2025 - 01 - 15);
        let discount_curve = create_test_discount_curve("USD-OIS", base_date);
        let forward_curve = create_test_forward_curve("USD-SOFR", base_date);
        let hazard_curve = create_test_hazard_curve("CORP-A", base_date);

        let current_market = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_curve)
            .insert_hazard(hazard_curve);

        // Create snapshot with new discount and hazard curves
        let new_discount = create_test_discount_curve("EUR-OIS", base_date);
        let new_hazard = create_test_hazard_curve("CORP-B", base_date);
        let snapshot = MarketSnapshot {
            discount_curves: vec![("EUR-OIS".into(), Arc::new(new_discount))]
                .into_iter()
                .collect(),
            hazard_curves: vec![("CORP-B".into(), Arc::new(new_hazard))]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        // Restore everything except forward curves
        let flags = CurveRestoreFlags::all() & !CurveRestoreFlags::FORWARD;
        let restored = MarketSnapshot::restore_market(&current_market, &snapshot, flags);

        // Should have new discount and hazard from snapshot
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_hazard("CORP-B").is_ok());
        // Original discount and hazard should be replaced
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_hazard("CORP-A").is_err());
        // Forward curve should be preserved (not in restore flags)
        assert!(restored.get_forward("USD-SOFR").is_ok());
    }

    #[test]
    fn test_restore_rates_curves_equivalence() {
        let base_date = date!(2025 - 01 - 15);

        // Build a market with multiple curve types
        let discount1 = create_test_discount_curve("USD-OIS", base_date);
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);
        let forward1 = create_test_forward_curve("USD-SOFR", base_date);
        let forward2 = create_test_forward_curve("EUR-ESTR", base_date);
        let hazard1 = create_test_hazard_curve("CORP-A", base_date);
        let inflation1 = create_test_inflation_curve("US-CPI", base_date);

        let market = MarketContext::new()
            .insert_discount(discount1)
            .insert_discount(discount2)
            .insert_forward(forward1)
            .insert_forward(forward2)
            .insert_hazard(hazard1)
            .insert_inflation(inflation1);

        // Extract rates snapshot
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::RATES);

        // Create a different market to restore into
        let hazard2 = create_test_hazard_curve("CORP-B", base_date);
        let inflation2 = create_test_inflation_curve("EU-HICP", base_date);
        let target_market = MarketContext::new()
            .insert_hazard(hazard2)
            .insert_inflation(inflation2);

        // Restore rates curves
        let restored =
            MarketSnapshot::restore_market(&target_market, &snapshot, CurveRestoreFlags::RATES);

        // Verify: should have rates curves from snapshot
        assert!(restored.get_discount("USD-OIS").is_ok());
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_forward("USD-SOFR").is_ok());
        assert!(restored.get_forward("EUR-ESTR").is_ok());

        // Verify: should preserve non-rates curves from target
        assert!(restored.get_hazard("CORP-B").is_ok());
        assert!(restored.get_inflation("EU-HICP").is_ok());

        // Verify: should NOT have original hazard/inflation from source market
        assert!(restored.get_hazard("CORP-A").is_err());
        assert!(restored.get_inflation("US-CPI").is_err());

        // Sanity check: restored market should match expectations
        assert!(restored.get_discount("USD-OIS").is_ok());
    }

    #[test]
    fn test_restore_credit_curves_equivalence() {
        let base_date = date!(2025 - 01 - 15);

        // Build a market with multiple curve types
        let discount1 = create_test_discount_curve("USD-OIS", base_date);
        let forward1 = create_test_forward_curve("USD-SOFR", base_date);
        let hazard1 = create_test_hazard_curve("CORP-A", base_date);
        let hazard2 = create_test_hazard_curve("CORP-B", base_date);
        let inflation1 = create_test_inflation_curve("US-CPI", base_date);

        let market = MarketContext::new()
            .insert_discount(discount1)
            .insert_forward(forward1)
            .insert_hazard(hazard1)
            .insert_hazard(hazard2)
            .insert_inflation(inflation1);

        // Extract credit snapshot
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::CREDIT);

        // Create a different market to restore into
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);
        let forward2 = create_test_forward_curve("EUR-ESTR", base_date);
        let target_market = MarketContext::new()
            .insert_discount(discount2)
            .insert_forward(forward2);

        // Restore credit curves
        let restored =
            MarketSnapshot::restore_market(&target_market, &snapshot, CurveRestoreFlags::CREDIT);

        // Verify: should have hazard curves from snapshot
        assert!(restored.get_hazard("CORP-A").is_ok());
        assert!(restored.get_hazard("CORP-B").is_ok());

        // Verify: should preserve non-credit curves from target
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_forward("EUR-ESTR").is_ok());

        // Verify: should NOT have original discount/forward from source market
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_forward("USD-SOFR").is_err());

        // Sanity check: restored market should match expectations
        assert!(restored.get_hazard("CORP-A").is_ok());
    }

    #[test]
    fn test_restore_inflation_curves_equivalence() {
        let base_date = date!(2025 - 01 - 15);

        // Build a market with multiple curve types
        let discount1 = create_test_discount_curve("USD-OIS", base_date);
        let hazard1 = create_test_hazard_curve("CORP-A", base_date);
        let inflation1 = create_test_inflation_curve("US-CPI", base_date);
        let inflation2 = create_test_inflation_curve("EU-HICP", base_date);

        let market = MarketContext::new()
            .insert_discount(discount1)
            .insert_hazard(hazard1)
            .insert_inflation(inflation1)
            .insert_inflation(inflation2);

        // Extract inflation snapshot
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::INFLATION);

        // Create a different market to restore into
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);
        let hazard2 = create_test_hazard_curve("CORP-B", base_date);
        let target_market = MarketContext::new()
            .insert_discount(discount2)
            .insert_hazard(hazard2);

        // Restore inflation curves
        let restored =
            MarketSnapshot::restore_market(&target_market, &snapshot, CurveRestoreFlags::INFLATION);

        // Verify: should have inflation curves from snapshot
        assert!(restored.get_inflation("US-CPI").is_ok());
        assert!(restored.get_inflation("EU-HICP").is_ok());

        // Verify: should preserve non-inflation curves from target
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_hazard("CORP-B").is_ok());

        // Verify: should NOT have original curves from source market
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_hazard("CORP-A").is_err());

        // Sanity check: restored market should match expectations
        assert!(restored.get_inflation("US-CPI").is_ok());
    }

    #[test]
    fn test_restore_correlations_equivalence() {
        let base_date = date!(2025 - 01 - 15);

        // Build a market with multiple curve types
        let discount1 = create_test_discount_curve("USD-OIS", base_date);
        let hazard1 = create_test_hazard_curve("CORP-A", base_date);
        let corr1 = create_test_base_correlation_curve("CDX-IG", base_date);
        let corr2 = create_test_base_correlation_curve("ITRAXX", base_date);

        let market = MarketContext::new()
            .insert_discount(discount1)
            .insert_hazard(hazard1)
            .insert_base_correlation(corr1)
            .insert_base_correlation(corr2);

        // Extract correlation snapshot
        let snapshot = MarketSnapshot::extract(&market, CurveRestoreFlags::CORRELATION);

        // Create a different market to restore into
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);
        let hazard2 = create_test_hazard_curve("CORP-B", base_date);
        let target_market = MarketContext::new()
            .insert_discount(discount2)
            .insert_hazard(hazard2);

        // Restore correlation curves
        let restored = MarketSnapshot::restore_market(
            &target_market,
            &snapshot,
            CurveRestoreFlags::CORRELATION,
        );

        // Verify: should have correlation curves from snapshot
        assert!(restored.get_base_correlation("CDX-IG").is_ok());
        assert!(restored.get_base_correlation("ITRAXX").is_ok());

        // Verify: should preserve non-correlation curves from target
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_hazard("CORP-B").is_ok());

        // Verify: should NOT have original curves from source market
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_hazard("CORP-A").is_err());

        // Sanity check: restored market should match expectations
        assert!(restored.get_base_correlation("CDX-IG").is_ok());
    }

    #[test]
    fn test_restore_equivalence_empty_markets() {
        let base_date = date!(2025 - 01 - 15);

        // Test with empty source market
        let empty_market = MarketContext::new();
        let snapshot = MarketSnapshot::extract(&empty_market, CurveRestoreFlags::RATES);
        let target =
            MarketContext::new().insert_discount(create_test_discount_curve("USD-OIS", base_date));

        let restored = MarketSnapshot::restore_market(&target, &snapshot, CurveRestoreFlags::RATES);

        // Should have removed all rates curves (snapshot was empty)
        assert!(restored.get_discount("USD-OIS").is_err());

        // Test with empty target market
        let source =
            MarketContext::new().insert_discount(create_test_discount_curve("USD-OIS", base_date));
        let snapshot2 = MarketSnapshot::extract(&source, CurveRestoreFlags::RATES);
        let empty_target = MarketContext::new();

        let restored2 =
            MarketSnapshot::restore_market(&empty_target, &snapshot2, CurveRestoreFlags::RATES);

        // Should have rates curves from snapshot
        assert!(restored2.get_discount("USD-OIS").is_ok());
    }

    #[test]
    fn test_restore_equivalence_mixed_curve_types() {
        let base_date = date!(2025 - 01 - 15);

        // Build a complex market with all curve types
        let discount1 = create_test_discount_curve("USD-OIS", base_date);
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);
        let forward1 = create_test_forward_curve("USD-SOFR", base_date);
        let hazard1 = create_test_hazard_curve("CORP-A", base_date);
        let inflation1 = create_test_inflation_curve("US-CPI", base_date);
        let corr1 = create_test_base_correlation_curve("CDX-IG", base_date);

        let market = MarketContext::new()
            .insert_discount(discount1)
            .insert_discount(discount2)
            .insert_forward(forward1)
            .insert_hazard(hazard1)
            .insert_inflation(inflation1)
            .insert_base_correlation(corr1);

        // Extract each type of snapshot
        let rates_snap = MarketSnapshot::extract(&market, CurveRestoreFlags::RATES);
        let credit_snap = MarketSnapshot::extract(&market, CurveRestoreFlags::CREDIT);
        let inflation_snap = MarketSnapshot::extract(&market, CurveRestoreFlags::INFLATION);
        let corr_snap = MarketSnapshot::extract(&market, CurveRestoreFlags::CORRELATION);

        // Build a different target market
        let target_discount = create_test_discount_curve("GBP-OIS", base_date);
        let target_hazard = create_test_hazard_curve("CORP-B", base_date);
        let target = MarketContext::new()
            .insert_discount(target_discount)
            .insert_hazard(target_hazard);

        // Restore rates curves
        let after_rates =
            MarketSnapshot::restore_market(&target, &rates_snap, CurveRestoreFlags::RATES);
        assert_eq!(
            after_rates
                .curve_ids()
                .filter(|id| after_rates.get_discount(id).is_ok())
                .count(),
            2
        );
        assert_eq!(
            after_rates
                .curve_ids()
                .filter(|id| after_rates.get_forward(id).is_ok())
                .count(),
            1
        );
        assert!(after_rates.get_hazard("CORP-B").is_ok()); // preserved

        // Restore credit curves on top of rates
        let after_credit =
            MarketSnapshot::restore_market(&after_rates, &credit_snap, CurveRestoreFlags::CREDIT);
        assert!(after_credit.get_discount("USD-OIS").is_ok()); // preserved from rates
        assert!(after_credit.get_discount("EUR-OIS").is_ok()); // preserved from rates
        assert!(after_credit.get_forward("USD-SOFR").is_ok()); // preserved from rates
        assert!(after_credit.get_hazard("CORP-A").is_ok()); // restored
        assert!(after_credit.get_hazard("CORP-B").is_err()); // replaced

        // Restore inflation curves
        let after_inflation = MarketSnapshot::restore_market(
            &after_credit,
            &inflation_snap,
            CurveRestoreFlags::INFLATION,
        );
        assert!(after_inflation.get_inflation("US-CPI").is_ok()); // restored

        // Restore correlation curves
        let final_market = MarketSnapshot::restore_market(
            &after_inflation,
            &corr_snap,
            CurveRestoreFlags::CORRELATION,
        );
        assert!(final_market.get_base_correlation("CDX-IG").is_ok()); // restored

        // Verify final state has all original curves except CORP-B
        assert!(final_market.get_discount("USD-OIS").is_ok());
        assert!(final_market.get_discount("EUR-OIS").is_ok());
        assert!(final_market.get_forward("USD-SOFR").is_ok());
        assert!(final_market.get_hazard("CORP-A").is_ok());
        assert!(final_market.get_inflation("US-CPI").is_ok());
        assert!(final_market.get_base_correlation("CDX-IG").is_ok());
        assert!(final_market.get_discount("GBP-OIS").is_err());
        assert!(final_market.get_hazard("CORP-B").is_err());
    }

    // ===== MarketExtractable Trait Tests =====

    #[test]
    fn test_market_extractable_rates_curves() {
        let base_date = date!(2025 - 01 - 15);
        let discount = create_test_discount_curve("USD-OIS", base_date);

        let market = MarketContext::new().insert_discount(discount);

        // Test trait method
        let snapshot = RatesCurvesSnapshot::extract(&market);
        assert_eq!(snapshot.discount_curves.len(), 1);
        assert!(snapshot.discount_curves.contains_key("USD-OIS"));

        // Test generic function
        let generic_snapshot: RatesCurvesSnapshot = extract(&market);
        assert_eq!(generic_snapshot.discount_curves.len(), 1);
        assert!(generic_snapshot.discount_curves.contains_key("USD-OIS"));
    }

    #[test]
    fn test_market_extractable_credit_curves() {
        let base_date = date!(2025 - 01 - 15);
        let hazard = create_test_hazard_curve("CORP-A", base_date);
        let market = MarketContext::new().insert_hazard(hazard);

        // Test trait method
        let snapshot = CreditCurvesSnapshot::extract(&market);
        assert_eq!(snapshot.hazard_curves.len(), 1);
        assert!(snapshot.hazard_curves.contains_key("CORP-A"));

        // Test generic function
        let generic_snapshot: CreditCurvesSnapshot = extract(&market);
        assert_eq!(generic_snapshot.hazard_curves.len(), 1);
    }

    #[test]
    fn test_market_extractable_inflation_curves() {
        let base_date = date!(2025 - 01 - 15);
        let inflation = create_test_inflation_curve("US-CPI", base_date);
        let market = MarketContext::new().insert_inflation(inflation);

        // Test trait method
        let snapshot = InflationCurvesSnapshot::extract(&market);
        assert_eq!(snapshot.inflation_curves.len(), 1);
        assert!(snapshot.inflation_curves.contains_key("US-CPI"));

        // Test generic function
        let generic_snapshot: InflationCurvesSnapshot = extract(&market);
        assert_eq!(generic_snapshot.inflation_curves.len(), 1);
    }

    #[test]
    fn test_market_extractable_correlations() {
        let base_date = date!(2025 - 01 - 15);
        let base_corr = create_test_base_correlation_curve("CDX-IG", base_date);
        let market = MarketContext::new().insert_base_correlation(base_corr);

        // Test trait method
        let snapshot = CorrelationsSnapshot::extract(&market);
        assert_eq!(snapshot.base_correlation_curves.len(), 1);
        assert!(snapshot.base_correlation_curves.contains_key("CDX-IG"));

        // Test generic function
        let generic_snapshot: CorrelationsSnapshot = extract(&market);
        assert_eq!(generic_snapshot.base_correlation_curves.len(), 1);
    }

    #[test]
    fn test_market_extractable_volatility() {
        // VolatilitySnapshot extracts from surfaces field which requires complex setup
        // Test with empty market to verify trait works
        let market = MarketContext::new();

        // Test trait method
        let snapshot = VolatilitySnapshot::extract(&market);
        assert!(snapshot.surfaces.is_empty());

        // Test generic function
        let generic_snapshot: VolatilitySnapshot = extract(&market);
        assert!(generic_snapshot.surfaces.is_empty());
    }

    #[test]
    fn test_market_extractable_scalars() {
        // Test with empty market to verify trait works
        let market = MarketContext::new();

        // Test trait method
        let snapshot = ScalarsSnapshot::extract(&market);
        assert_eq!(snapshot.prices.len(), 0);
        assert_eq!(snapshot.series.len(), 0);
        assert_eq!(snapshot.inflation_indices.len(), 0);
        assert_eq!(snapshot.dividends.len(), 0);

        // Test generic function
        let generic_snapshot: ScalarsSnapshot = extract(&market);
        assert_eq!(generic_snapshot.prices.len(), 0);
    }

    #[test]
    fn test_trait_vs_generic_extract_equivalence() {
        // Verify that trait-based extraction matches the generic helper
        let base_date = date!(2025 - 01 - 15);
        let discount = create_test_discount_curve("USD-OIS", base_date);
        let market = MarketContext::new().insert_discount(discount);

        let direct = RatesCurvesSnapshot::extract(&market);
        let generic: RatesCurvesSnapshot = extract(&market);

        assert_eq!(direct.discount_curves.len(), generic.discount_curves.len());
        assert_eq!(direct.forward_curves.len(), generic.forward_curves.len());

        // Verify curve IDs match
        for id in direct.discount_curves.keys() {
            assert!(generic.discount_curves.contains_key(id));
        }
    }

    #[test]
    fn test_generic_extract_with_type_inference() {
        let base_date = date!(2025 - 01 - 15);
        let discount = create_test_discount_curve("USD-OIS", base_date);
        let market = MarketContext::new().insert_discount(discount);

        // Test that type inference works correctly
        let _rates: RatesCurvesSnapshot = extract(&market);
        let _volatility: VolatilitySnapshot = extract(&market);
        let _scalars: ScalarsSnapshot = extract(&market);

        // If this compiles, type inference is working correctly
    }

    #[test]
    fn test_market_extractable_multiple_curves() {
        use finstack_core::market_data::term_structures::ForwardCurve;

        let base_date = date!(2025 - 01 - 15);
        let discount1 = create_test_discount_curve("USD-OIS", base_date);
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);

        let forward = ForwardCurve::builder("USD-SOFR", 0.25) // 3-month forward
            .base_date(base_date)
            .knots(vec![(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("ForwardCurve builder should succeed");

        let market = MarketContext::new()
            .insert_discount(discount1)
            .insert_discount(discount2)
            .insert_forward(forward);

        // Extract rates curves
        let snapshot = RatesCurvesSnapshot::extract(&market);

        // Verify we got all curves
        assert_eq!(snapshot.discount_curves.len(), 2);
        assert_eq!(snapshot.forward_curves.len(), 1);
        assert!(snapshot.discount_curves.contains_key("USD-OIS"));
        assert!(snapshot.discount_curves.contains_key("EUR-OIS"));
        assert!(snapshot.forward_curves.contains_key("USD-SOFR"));
    }
}
