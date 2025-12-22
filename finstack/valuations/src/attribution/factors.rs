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
//! The original `restore_*_curves()` functions are maintained as thin wrappers for
//! backward compatibility, but internally they delegate to the unified implementation.
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
//! use finstack_valuations::attribution::factors::{
//!     extract_rates_curves, restore_rates_curves, CurveRestoreFlags, MarketSnapshot
//! };
//! use finstack_core::market_data::context::MarketContext;
//!
//! // Original API (backward compatible)
//! let market_t0 = MarketContext::new();
//! // ... populate market_t0 with curves
//!
//! let rates_snapshot = extract_rates_curves(&market_t0);
//! let market_t1 = MarketContext::new(); // market with moved curves
//! // ... populate market_t1 with shocked curves
//!
//! // Restore t0 rates while keeping t1 credit/inflation/correlation curves
//! let mixed_market = restore_rates_curves(&market_t1, &rates_snapshot);
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
//! use finstack_valuations::attribution::factors::{
//!     CurveRestoreFlags, MarketSnapshot, extract_rates_curves, extract_credit_curves
//! };
//! use finstack_core::market_data::context::MarketContext;
//!
//! // Start with markets at t0 and t1
//! let market_t0 = MarketContext::new();
//! let market_t1 = MarketContext::new();
//! // ... populate both markets
//!
//! // Attribute P&L to rates move
//! let rates_snapshot = extract_rates_curves(&market_t0);
//! let market_only_rates_moved = MarketSnapshot::restore_market(
//!     &market_t1,
//!     &MarketSnapshot {
//!         discount_curves: rates_snapshot.discount_curves,
//!         forward_curves: rates_snapshot.forward_curves,
//!         ..Default::default()
//!     },
//!     CurveRestoreFlags::RATES
//! );
//! // Price with market_only_rates_moved to isolate rates P&L
//!
//! // Attribute P&L to credit move
//! let credit_snapshot = extract_credit_curves(&market_t0);
//! let market_only_credit_moved = MarketSnapshot::restore_market(
//!     &market_t1,
//!     &MarketSnapshot {
//!         hazard_curves: credit_snapshot.hazard_curves,
//!         ..Default::default()
//!     },
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
//! The old `extract_*_curves()` functions are deprecated in favor of this trait-based
//! approach, which provides better type inference and reduces the module's public API
//! surface.
//!
//! # See Also
//!
//! - [`crate::attribution::parallel`] - Parallel attribution using this module
//! - [`crate::attribution::waterfall`] - Waterfall attribution using this module

use bitflags::bitflags;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::dividends::DividendSchedule;
use finstack_core::market_data::scalars::inflation_index::InflationIndex;
use finstack_core::market_data::scalars::{MarketScalar, ScalarTimeSeries};
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::money::fx::FxMatrix;
use finstack_core::types::CurveId;
use hashbrown::HashMap;
use std::sync::Arc;

bitflags! {
    /// Flags indicating which curve families to restore from snapshot vs. preserve from market.
    ///
    /// This enum is used to control which curve types should be restored from a snapshot
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
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CurveRestoreFlags: u8 {
        /// Restore discount curves from snapshot
        const DISCOUNT    = 0b0000_0001;
        /// Restore forward curves from snapshot
        const FORWARD     = 0b0000_0010;
        /// Restore hazard curves from snapshot
        const HAZARD      = 0b0000_0100;
        /// Restore inflation curves from snapshot
        const INFLATION   = 0b0000_1000;
        /// Restore base correlation curves from snapshot
        const CORRELATION = 0b0001_0000;

        /// Convenience combination: restore both discount and forward curves (rates family)
        const RATES  = Self::DISCOUNT.bits() | Self::FORWARD.bits();
        /// Convenience combination: restore hazard curves (credit family)
        const CREDIT = Self::HAZARD.bits();
    }
}

/// Snapshot of all discount and forward curves from a market context.
#[derive(Clone, Debug)]
pub struct RatesCurvesSnapshot {
    /// Discount curves indexed by curve ID
    pub discount_curves: HashMap<CurveId, Arc<DiscountCurve>>,
    /// Forward curves indexed by curve ID
    pub forward_curves: HashMap<CurveId, Arc<ForwardCurve>>,
}

/// Snapshot of all credit hazard curves from a market context.
#[derive(Clone, Debug)]
pub struct CreditCurvesSnapshot {
    /// Hazard curves indexed by curve ID
    pub hazard_curves: HashMap<CurveId, Arc<HazardCurve>>,
}

/// Snapshot of all inflation curves from a market context.
#[derive(Clone, Debug)]
pub struct InflationCurvesSnapshot {
    /// Inflation curves indexed by curve ID
    pub inflation_curves: HashMap<CurveId, Arc<InflationCurve>>,
}

/// Snapshot of all base correlation curves from a market context.
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
        for (_id, curve) in &preserved.discount_curves {
            new_market = new_market.insert_discount((**curve).clone());
        }
        for (_id, curve) in &preserved.forward_curves {
            new_market = new_market.insert_forward((**curve).clone());
        }
        for (_id, curve) in &preserved.hazard_curves {
            new_market = new_market.insert_hazard((**curve).clone());
        }
        for (_id, curve) in &preserved.inflation_curves {
            new_market = new_market.insert_inflation((**curve).clone());
        }
        for (_id, curve) in &preserved.base_correlation_curves {
            new_market = new_market.insert_base_correlation((**curve).clone());
        }

        // Insert snapshot curves (these ARE being restored)
        // Only insert curves that were actually in the snapshot
        for (_id, curve) in &snapshot.discount_curves {
            new_market = new_market.insert_discount((**curve).clone());
        }
        for (_id, curve) in &snapshot.forward_curves {
            new_market = new_market.insert_forward((**curve).clone());
        }
        for (_id, curve) in &snapshot.hazard_curves {
            new_market = new_market.insert_hazard((**curve).clone());
        }
        for (_id, curve) in &snapshot.inflation_curves {
            new_market = new_market.insert_inflation((**curve).clone());
        }
        for (_id, curve) in &snapshot.base_correlation_curves {
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
pub fn extract<T: MarketExtractable>(market: &MarketContext) -> T {
    T::extract(market)
}

// Implement MarketExtractable for all snapshot types
impl MarketExtractable for RatesCurvesSnapshot {
    fn extract(market: &MarketContext) -> Self {
        let mut discount_curves = HashMap::new();
        let mut forward_curves = HashMap::new();

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
        let mut hazard_curves = HashMap::new();

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
        let mut inflation_curves = HashMap::new();

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
        let mut base_correlation_curves = HashMap::new();

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

/// Extract all discount and forward curves from a market context.
///
/// This function is now a thin wrapper around the `MarketExtractable` trait.
/// Consider using `RatesCurvesSnapshot::extract(market)` or `extract::<RatesCurvesSnapshot>(market)` instead.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all rates curves.
///
/// # Migration
///
/// Use the trait-based approach instead:
/// ```rust,no_run
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_valuations::attribution::factors::{
///     extract, extract_rates_curves, MarketExtractable, RatesCurvesSnapshot,
/// };
///
/// let market = MarketContext::new();
///
/// // Old way (deprecated)
/// let _snapshot = extract_rates_curves(&market);
///
/// // New way (recommended)
/// let _snapshot = RatesCurvesSnapshot::extract(&market);
/// let _snapshot = extract::<RatesCurvesSnapshot>(&market);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use RatesCurvesSnapshot::extract() or extract::<RatesCurvesSnapshot>() instead"
)]
pub fn extract_rates_curves(market: &MarketContext) -> RatesCurvesSnapshot {
    RatesCurvesSnapshot::extract(market)
}

/// Extract all credit hazard curves from a market context.
///
/// This function is now a thin wrapper around the `MarketExtractable` trait.
/// Consider using `CreditCurvesSnapshot::extract(market)` or `extract::<CreditCurvesSnapshot>(market)` instead.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all hazard curves.
///
/// # Migration
///
/// Use the trait-based approach instead:
/// ```rust,no_run
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_valuations::attribution::factors::{
///     extract, extract_credit_curves, CreditCurvesSnapshot, MarketExtractable,
/// };
///
/// let market = MarketContext::new();
///
/// // Old way (deprecated)
/// let _snapshot = extract_credit_curves(&market);
///
/// // New way (recommended)
/// let _snapshot = CreditCurvesSnapshot::extract(&market);
/// let _snapshot = extract::<CreditCurvesSnapshot>(&market);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use CreditCurvesSnapshot::extract() or extract::<CreditCurvesSnapshot>() instead"
)]
pub fn extract_credit_curves(market: &MarketContext) -> CreditCurvesSnapshot {
    CreditCurvesSnapshot::extract(market)
}

/// Extract all inflation curves from a market context.
///
/// This function is now a thin wrapper around the `MarketExtractable` trait.
/// Consider using `InflationCurvesSnapshot::extract(market)` or `extract::<InflationCurvesSnapshot>(market)` instead.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all inflation curves.
///
/// # Migration
///
/// Use the trait-based approach instead:
/// ```rust,no_run
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_valuations::attribution::factors::{
///     extract, extract_inflation_curves, InflationCurvesSnapshot, MarketExtractable,
/// };
///
/// let market = MarketContext::new();
///
/// // Old way (deprecated)
/// let _snapshot = extract_inflation_curves(&market);
///
/// // New way (recommended)
/// let _snapshot = InflationCurvesSnapshot::extract(&market);
/// let _snapshot = extract::<InflationCurvesSnapshot>(&market);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use InflationCurvesSnapshot::extract() or extract::<InflationCurvesSnapshot>() instead"
)]
pub fn extract_inflation_curves(market: &MarketContext) -> InflationCurvesSnapshot {
    InflationCurvesSnapshot::extract(market)
}

/// Extract all base correlation curves from a market context.
///
/// This function is now a thin wrapper around the `MarketExtractable` trait.
/// Consider using `CorrelationsSnapshot::extract(market)` or `extract::<CorrelationsSnapshot>(market)` instead.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all correlation curves.
///
/// # Migration
///
/// Use the trait-based approach instead:
/// ```rust,no_run
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_valuations::attribution::factors::{
///     extract, extract_correlations, CorrelationsSnapshot, MarketExtractable,
/// };
///
/// let market = MarketContext::new();
///
/// // Old way (deprecated)
/// let _snapshot = extract_correlations(&market);
///
/// // New way (recommended)
/// let _snapshot = CorrelationsSnapshot::extract(&market);
/// let _snapshot = extract::<CorrelationsSnapshot>(&market);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use CorrelationsSnapshot::extract() or extract::<CorrelationsSnapshot>() instead"
)]
pub fn extract_correlations(market: &MarketContext) -> CorrelationsSnapshot {
    CorrelationsSnapshot::extract(market)
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

/// Extract volatility surfaces from a market context.
///
/// This function is now a thin wrapper around the `MarketExtractable` trait.
/// Consider using `VolatilitySnapshot::extract(market)` or `extract::<VolatilitySnapshot>(market)` instead.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all volatility surfaces.
///
/// # Migration
///
/// Use the trait-based approach instead:
/// ```rust,no_run
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_valuations::attribution::factors::{
///     extract, extract_volatility, MarketExtractable, VolatilitySnapshot,
/// };
///
/// let market = MarketContext::new();
///
/// // Old way (deprecated)
/// let _snapshot = extract_volatility(&market);
///
/// // New way (recommended)
/// let _snapshot = VolatilitySnapshot::extract(&market);
/// let _snapshot = extract::<VolatilitySnapshot>(&market);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use VolatilitySnapshot::extract() or extract::<VolatilitySnapshot>() instead"
)]
pub fn extract_volatility(market: &MarketContext) -> VolatilitySnapshot {
    VolatilitySnapshot::extract(market)
}

/// Extract market scalars from a market context.
///
/// This function is now a thin wrapper around the `MarketExtractable` trait.
/// Consider using `ScalarsSnapshot::extract(market)` or `extract::<ScalarsSnapshot>(market)` instead.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all market scalars.
///
/// # Migration
///
/// Use the trait-based approach instead:
/// ```rust,no_run
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_valuations::attribution::factors::{
///     extract, extract_scalars, MarketExtractable, ScalarsSnapshot,
/// };
///
/// let market = MarketContext::new();
///
/// // Old way (deprecated)
/// let _snapshot = extract_scalars(&market);
///
/// // New way (recommended)
/// let _snapshot = ScalarsSnapshot::extract(&market);
/// let _snapshot = extract::<ScalarsSnapshot>(&market);
/// ```
#[deprecated(
    since = "0.1.0",
    note = "Use ScalarsSnapshot::extract() or extract::<ScalarsSnapshot>() instead"
)]
pub fn extract_scalars(market: &MarketContext) -> ScalarsSnapshot {
    ScalarsSnapshot::extract(market)
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

/// Replace rates curves in a market context with curves from a snapshot.
///
/// This is a thin wrapper around [`MarketSnapshot::restore_market`] that maintains
/// backward compatibility with the original API.
///
/// # Arguments
///
/// * `market` - Market context to modify
/// * `snapshot` - Snapshot of rates curves to restore
///
/// # Returns
///
/// New market context with replaced rates curves while preserving all other data.
pub fn restore_rates_curves(
    market: &MarketContext,
    snapshot: &RatesCurvesSnapshot,
) -> MarketContext {
    // Convert specific snapshot to unified snapshot
    let unified = MarketSnapshot {
        discount_curves: snapshot.discount_curves.clone(),
        forward_curves: snapshot.forward_curves.clone(),
        ..Default::default()
    };

    // Use unified restore function with RATES flag
    MarketSnapshot::restore_market(market, &unified, CurveRestoreFlags::RATES)
}

/// Replace credit curves in a market context with curves from a snapshot.
///
/// This is a thin wrapper around [`MarketSnapshot::restore_market`] that maintains
/// backward compatibility with the original API.
///
/// # Arguments
///
/// * `market` - Market context to modify
/// * `snapshot` - Snapshot of credit curves to restore
///
/// # Returns
///
/// New market context with replaced credit curves while preserving all other data.
pub fn restore_credit_curves(
    market: &MarketContext,
    snapshot: &CreditCurvesSnapshot,
) -> MarketContext {
    // Convert specific snapshot to unified snapshot
    let unified = MarketSnapshot {
        hazard_curves: snapshot.hazard_curves.clone(),
        ..Default::default()
    };

    // Use unified restore function with CREDIT flag
    MarketSnapshot::restore_market(market, &unified, CurveRestoreFlags::CREDIT)
}

/// Replace inflation curves in a market context with curves from a snapshot.
///
/// This is a thin wrapper around [`MarketSnapshot::restore_market`] that maintains
/// backward compatibility with the original API.
///
/// # Arguments
///
/// * `market` - Market context to modify
/// * `snapshot` - Snapshot of inflation curves to restore
///
/// # Returns
///
/// New market context with replaced inflation curves while preserving all other data.
pub fn restore_inflation_curves(
    market: &MarketContext,
    snapshot: &InflationCurvesSnapshot,
) -> MarketContext {
    // Convert specific snapshot to unified snapshot
    let unified = MarketSnapshot {
        inflation_curves: snapshot.inflation_curves.clone(),
        ..Default::default()
    };

    // Use unified restore function with INFLATION flag
    MarketSnapshot::restore_market(market, &unified, CurveRestoreFlags::INFLATION)
}

/// Replace correlation curves in a market context with curves from a snapshot.
///
/// This is a thin wrapper around [`MarketSnapshot::restore_market`] that maintains
/// backward compatibility with the original API.
///
/// # Arguments
///
/// * `market` - Market context to modify
/// * `snapshot` - Snapshot of correlation curves to restore
///
/// # Returns
///
/// New market context with replaced correlation curves while preserving all other data.
pub fn restore_correlations(
    market: &MarketContext,
    snapshot: &CorrelationsSnapshot,
) -> MarketContext {
    // Convert specific snapshot to unified snapshot
    let unified = MarketSnapshot {
        base_correlation_curves: snapshot.base_correlation_curves.clone(),
        ..Default::default()
    };

    // Use unified restore function with CORRELATION flag
    MarketSnapshot::restore_market(market, &unified, CurveRestoreFlags::CORRELATION)
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
    for (_id, series) in &snapshot.series {
        new_market.set_series_mut(series.clone());
    }
    for (id, index) in &snapshot.inflation_indices {
        new_market.set_inflation_index_mut(id.as_str(), Arc::clone(index));
    }
    for (_id, schedule) in &snapshot.dividends {
        new_market.set_dividends_mut(Arc::clone(schedule));
    }

    new_market
}

#[cfg(test)]
#[allow(deprecated)] // TODO: Migrate tests to use trait-based extraction
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
        let snapshot = extract_rates_curves(&market);
        assert_eq!(snapshot.discount_curves.len(), 2);

        // Create empty market and restore
        let empty_market = MarketContext::new();
        let restored = restore_rates_curves(&empty_market, &snapshot);

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
            .points(vec![
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

    // ============================================================================
    // Equivalence Tests: Verify new unified implementation produces identical
    // results to the old implementation behavior
    // ============================================================================

    /// Helper function to assert two market contexts have equivalent curve structure.
    ///
    /// This compares:
    /// - Curve counts by type
    /// - Curve IDs present
    /// - Discount factor values at sample dates (for discount curves)
    /// - FX provider presence
    fn assert_market_contexts_equal(ctx1: &MarketContext, ctx2: &MarketContext, label: &str) {
        // Count curves by type
        let count_discount_1: usize = ctx1
            .curve_ids()
            .filter(|id| ctx1.get_discount(id).is_ok())
            .count();
        let count_discount_2: usize = ctx2
            .curve_ids()
            .filter(|id| ctx2.get_discount(id).is_ok())
            .count();
        assert_eq!(
            count_discount_1, count_discount_2,
            "{}: discount curve counts differ",
            label
        );

        let count_forward_1: usize = ctx1
            .curve_ids()
            .filter(|id| ctx1.get_forward(id).is_ok())
            .count();
        let count_forward_2: usize = ctx2
            .curve_ids()
            .filter(|id| ctx2.get_forward(id).is_ok())
            .count();
        assert_eq!(
            count_forward_1, count_forward_2,
            "{}: forward curve counts differ",
            label
        );

        let count_hazard_1: usize = ctx1
            .curve_ids()
            .filter(|id| ctx1.get_hazard(id).is_ok())
            .count();
        let count_hazard_2: usize = ctx2
            .curve_ids()
            .filter(|id| ctx2.get_hazard(id).is_ok())
            .count();
        assert_eq!(
            count_hazard_1, count_hazard_2,
            "{}: hazard curve counts differ",
            label
        );

        let count_inflation_1: usize = ctx1
            .curve_ids()
            .filter(|id| ctx1.get_inflation(id).is_ok())
            .count();
        let count_inflation_2: usize = ctx2
            .curve_ids()
            .filter(|id| ctx2.get_inflation(id).is_ok())
            .count();
        assert_eq!(
            count_inflation_1, count_inflation_2,
            "{}: inflation curve counts differ",
            label
        );

        let count_corr_1: usize = ctx1
            .curve_ids()
            .filter(|id| ctx1.get_base_correlation(id).is_ok())
            .count();
        let count_corr_2: usize = ctx2
            .curve_ids()
            .filter(|id| ctx2.get_base_correlation(id).is_ok())
            .count();
        assert_eq!(
            count_corr_1, count_corr_2,
            "{}: base correlation curve counts differ",
            label
        );

        // Compare curve IDs
        let mut ids1: Vec<_> = ctx1.curve_ids().map(|id| id.to_string()).collect();
        let mut ids2: Vec<_> = ctx2.curve_ids().map(|id| id.to_string()).collect();
        ids1.sort();
        ids2.sort();
        assert_eq!(ids1, ids2, "{}: curve IDs differ", label);

        // Compare discount factor values at sample dates for discount curves
        let sample_dates = vec![
            date!(2025 - 01 - 15),
            date!(2025 - 06 - 15),
            date!(2026 - 01 - 15),
            date!(2030 - 01 - 15),
        ];

        for curve_id in ctx1.curve_ids() {
            if let (Ok(curve1), Ok(curve2)) =
                (ctx1.get_discount(curve_id), ctx2.get_discount(curve_id))
            {
                for &sample_date in &sample_dates {
                    // Use try_df_on_date_curve which uses the curve's own day count
                    let df1 = curve1.try_df_on_date_curve(sample_date).unwrap_or(1.0);
                    let df2 = curve2.try_df_on_date_curve(sample_date).unwrap_or(1.0);
                    assert!(
                        (df1 - df2).abs() < 1e-10,
                        "{}: discount factor mismatch for curve {} at {:?}: {} vs {}",
                        label,
                        curve_id,
                        sample_date,
                        df1,
                        df2
                    );
                }
            }
        }

        // Compare FX provider presence
        assert_eq!(
            ctx1.fx.is_some(),
            ctx2.fx.is_some(),
            "{}: FX provider presence differs",
            label
        );
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
        let snapshot = extract_rates_curves(&market);

        // Create a different market to restore into
        let hazard2 = create_test_hazard_curve("CORP-B", base_date);
        let inflation2 = create_test_inflation_curve("EU-HICP", base_date);
        let target_market = MarketContext::new()
            .insert_hazard(hazard2)
            .insert_inflation(inflation2);

        // Restore using wrapper function
        let restored = restore_rates_curves(&target_market, &snapshot);

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

        // Create expected result using unified approach for comparison
        let unified_snapshot = MarketSnapshot {
            discount_curves: snapshot.discount_curves.clone(),
            forward_curves: snapshot.forward_curves.clone(),
            ..Default::default()
        };
        let expected = MarketSnapshot::restore_market(
            &target_market,
            &unified_snapshot,
            CurveRestoreFlags::RATES,
        );

        assert_market_contexts_equal(&restored, &expected, "restore_rates_curves equivalence");
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
        let snapshot = extract_credit_curves(&market);

        // Create a different market to restore into
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);
        let forward2 = create_test_forward_curve("EUR-ESTR", base_date);
        let target_market = MarketContext::new()
            .insert_discount(discount2)
            .insert_forward(forward2);

        // Restore using wrapper function
        let restored = restore_credit_curves(&target_market, &snapshot);

        // Verify: should have hazard curves from snapshot
        assert!(restored.get_hazard("CORP-A").is_ok());
        assert!(restored.get_hazard("CORP-B").is_ok());

        // Verify: should preserve non-credit curves from target
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_forward("EUR-ESTR").is_ok());

        // Verify: should NOT have original discount/forward from source market
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_forward("USD-SOFR").is_err());

        // Create expected result using unified approach for comparison
        let unified_snapshot = MarketSnapshot {
            hazard_curves: snapshot.hazard_curves.clone(),
            ..Default::default()
        };
        let expected = MarketSnapshot::restore_market(
            &target_market,
            &unified_snapshot,
            CurveRestoreFlags::CREDIT,
        );

        assert_market_contexts_equal(&restored, &expected, "restore_credit_curves equivalence");
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
        let snapshot = extract_inflation_curves(&market);

        // Create a different market to restore into
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);
        let hazard2 = create_test_hazard_curve("CORP-B", base_date);
        let target_market = MarketContext::new()
            .insert_discount(discount2)
            .insert_hazard(hazard2);

        // Restore using wrapper function
        let restored = restore_inflation_curves(&target_market, &snapshot);

        // Verify: should have inflation curves from snapshot
        assert!(restored.get_inflation("US-CPI").is_ok());
        assert!(restored.get_inflation("EU-HICP").is_ok());

        // Verify: should preserve non-inflation curves from target
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_hazard("CORP-B").is_ok());

        // Verify: should NOT have original curves from source market
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_hazard("CORP-A").is_err());

        // Create expected result using unified approach for comparison
        let unified_snapshot = MarketSnapshot {
            inflation_curves: snapshot.inflation_curves.clone(),
            ..Default::default()
        };
        let expected = MarketSnapshot::restore_market(
            &target_market,
            &unified_snapshot,
            CurveRestoreFlags::INFLATION,
        );

        assert_market_contexts_equal(&restored, &expected, "restore_inflation_curves equivalence");
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
        let snapshot = extract_correlations(&market);

        // Create a different market to restore into
        let discount2 = create_test_discount_curve("EUR-OIS", base_date);
        let hazard2 = create_test_hazard_curve("CORP-B", base_date);
        let target_market = MarketContext::new()
            .insert_discount(discount2)
            .insert_hazard(hazard2);

        // Restore using wrapper function
        let restored = restore_correlations(&target_market, &snapshot);

        // Verify: should have correlation curves from snapshot
        assert!(restored.get_base_correlation("CDX-IG").is_ok());
        assert!(restored.get_base_correlation("ITRAXX").is_ok());

        // Verify: should preserve non-correlation curves from target
        assert!(restored.get_discount("EUR-OIS").is_ok());
        assert!(restored.get_hazard("CORP-B").is_ok());

        // Verify: should NOT have original curves from source market
        assert!(restored.get_discount("USD-OIS").is_err());
        assert!(restored.get_hazard("CORP-A").is_err());

        // Create expected result using unified approach for comparison
        let unified_snapshot = MarketSnapshot {
            base_correlation_curves: snapshot.base_correlation_curves.clone(),
            ..Default::default()
        };
        let expected = MarketSnapshot::restore_market(
            &target_market,
            &unified_snapshot,
            CurveRestoreFlags::CORRELATION,
        );

        assert_market_contexts_equal(&restored, &expected, "restore_correlations equivalence");
    }

    #[test]
    fn test_restore_equivalence_empty_markets() {
        let base_date = date!(2025 - 01 - 15);

        // Test with empty source market
        let empty_market = MarketContext::new();
        let snapshot = extract_rates_curves(&empty_market);
        let target =
            MarketContext::new().insert_discount(create_test_discount_curve("USD-OIS", base_date));

        let restored = restore_rates_curves(&target, &snapshot);

        // Should have removed all rates curves (snapshot was empty)
        assert!(restored.get_discount("USD-OIS").is_err());

        // Test with empty target market
        let source =
            MarketContext::new().insert_discount(create_test_discount_curve("USD-OIS", base_date));
        let snapshot2 = extract_rates_curves(&source);
        let empty_target = MarketContext::new();

        let restored2 = restore_rates_curves(&empty_target, &snapshot2);

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
        let rates_snap = extract_rates_curves(&market);
        let credit_snap = extract_credit_curves(&market);
        let inflation_snap = extract_inflation_curves(&market);
        let corr_snap = extract_correlations(&market);

        // Build a different target market
        let target_discount = create_test_discount_curve("GBP-OIS", base_date);
        let target_hazard = create_test_hazard_curve("CORP-B", base_date);
        let target = MarketContext::new()
            .insert_discount(target_discount)
            .insert_hazard(target_hazard);

        // Restore rates curves
        let after_rates = restore_rates_curves(&target, &rates_snap);
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
        let after_credit = restore_credit_curves(&after_rates, &credit_snap);
        assert!(after_credit.get_discount("USD-OIS").is_ok()); // preserved from rates
        assert!(after_credit.get_discount("EUR-OIS").is_ok()); // preserved from rates
        assert!(after_credit.get_forward("USD-SOFR").is_ok()); // preserved from rates
        assert!(after_credit.get_hazard("CORP-A").is_ok()); // restored
        assert!(after_credit.get_hazard("CORP-B").is_err()); // replaced

        // Restore inflation curves
        let after_inflation = restore_inflation_curves(&after_credit, &inflation_snap);
        assert!(after_inflation.get_inflation("US-CPI").is_ok()); // restored

        // Restore correlation curves
        let final_market = restore_correlations(&after_inflation, &corr_snap);
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
    fn test_trait_vs_function_equivalence() {
        // Verify that trait-based extraction produces identical results to function calls
        let base_date = date!(2025 - 01 - 15);
        let discount = create_test_discount_curve("USD-OIS", base_date);
        let market = MarketContext::new().insert_discount(discount);

        // Compare old function vs trait method
        let function_result = extract_rates_curves(&market);
        let trait_result = RatesCurvesSnapshot::extract(&market);

        assert_eq!(
            function_result.discount_curves.len(),
            trait_result.discount_curves.len()
        );
        assert_eq!(
            function_result.forward_curves.len(),
            trait_result.forward_curves.len()
        );

        // Verify curve IDs match
        for (id, _) in &function_result.discount_curves {
            assert!(trait_result.discount_curves.contains_key(id));
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
