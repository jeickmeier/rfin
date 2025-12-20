//! Factor decomposition logic for P&L attribution.
//!
//! Provides functions to freeze/restore specific market factors while
//! manipulating MarketContext for attribution analysis.

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
            new_market = new_market.insert_discount(Arc::clone(curve));
        }
        for (_id, curve) in &preserved.forward_curves {
            new_market = new_market.insert_forward(Arc::clone(curve));
        }
        for (_id, curve) in &preserved.hazard_curves {
            new_market = new_market.insert_hazard(Arc::clone(curve));
        }
        for (_id, curve) in &preserved.inflation_curves {
            new_market = new_market.insert_inflation(Arc::clone(curve));
        }
        for (_id, curve) in &preserved.base_correlation_curves {
            new_market = new_market.insert_base_correlation(Arc::clone(curve));
        }

        // Insert snapshot curves (these ARE being restored)
        // Only insert curves that were actually in the snapshot
        for (_id, curve) in &snapshot.discount_curves {
            new_market = new_market.insert_discount(Arc::clone(curve));
        }
        for (_id, curve) in &snapshot.forward_curves {
            new_market = new_market.insert_forward(Arc::clone(curve));
        }
        for (_id, curve) in &snapshot.hazard_curves {
            new_market = new_market.insert_hazard(Arc::clone(curve));
        }
        for (_id, curve) in &snapshot.inflation_curves {
            new_market = new_market.insert_inflation(Arc::clone(curve));
        }
        for (_id, curve) in &snapshot.base_correlation_curves {
            new_market = new_market.insert_base_correlation(Arc::clone(curve));
        }

        // Always preserve FX, surfaces, and scalars from current market
        if let Some(fx) = &current_market.fx {
            new_market.insert_fx_mut(Arc::clone(fx));
        }
        new_market.surfaces = current_market.surfaces.clone();
        copy_scalars(current_market, &mut new_market);

        new_market
    }
}

/// Extract all discount and forward curves from a market context.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all rates curves.
pub fn extract_rates_curves(market: &MarketContext) -> RatesCurvesSnapshot {
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

/// Extract all credit hazard curves from a market context.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all hazard curves.
pub fn extract_credit_curves(market: &MarketContext) -> CreditCurvesSnapshot {
    let mut hazard_curves = HashMap::new();

    for curve_id in market.curve_ids() {
        if let Ok(hazard) = market.get_hazard(curve_id) {
            hazard_curves.insert(curve_id.clone(), hazard);
        }
    }

    CreditCurvesSnapshot { hazard_curves }
}

/// Extract all inflation curves from a market context.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all inflation curves.
pub fn extract_inflation_curves(market: &MarketContext) -> InflationCurvesSnapshot {
    let mut inflation_curves = HashMap::new();

    for curve_id in market.curve_ids() {
        if let Ok(inflation) = market.get_inflation(curve_id) {
            inflation_curves.insert(curve_id.clone(), inflation);
        }
    }

    InflationCurvesSnapshot { inflation_curves }
}

/// Extract all base correlation curves from a market context.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all correlation curves.
pub fn extract_correlations(market: &MarketContext) -> CorrelationsSnapshot {
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
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all volatility surfaces.
pub fn extract_volatility(market: &MarketContext) -> VolatilitySnapshot {
    VolatilitySnapshot {
        surfaces: market.surfaces.clone(),
    }
}

/// Extract market scalars from a market context.
///
/// # Arguments
///
/// * `market` - Market context to extract from
///
/// # Returns
///
/// Snapshot containing all market scalars.
pub fn extract_scalars(market: &MarketContext) -> ScalarsSnapshot {
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
            new_market = new_market.insert_discount(discount);
        } else if let Ok(forward) = market.get_forward(curve_id) {
            new_market = new_market.insert_forward(forward);
        } else if let Ok(hazard) = market.get_hazard(curve_id) {
            new_market = new_market.insert_hazard(hazard);
        } else if let Ok(inflation) = market.get_inflation(curve_id) {
            new_market = new_market.insert_inflation(inflation);
        } else if let Ok(base_corr) = market.get_base_correlation(curve_id) {
            new_market = new_market.insert_base_correlation(base_corr);
        }
    }

    // Copy FX and surfaces
    if let Some(fx) = &market.fx {
        new_market.insert_fx_mut(Arc::clone(fx));
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
        let restored = MarketSnapshot::restore_market(
            &current_market,
            &snapshot,
            CurveRestoreFlags::DISCOUNT,
        );

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
        let restored = MarketSnapshot::restore_market(
            &current_market,
            &snapshot,
            CurveRestoreFlags::RATES,
        );

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
        let restored = MarketSnapshot::restore_market(
            &current_market,
            &snapshot,
            CurveRestoreFlags::CREDIT,
        );

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
        let restored = MarketSnapshot::restore_market(
            &current_market,
            &snapshot,
            CurveRestoreFlags::RATES,
        );

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
        let restored = MarketSnapshot::restore_market(
            &current_market,
            &snapshot,
            CurveRestoreFlags::RATES,
        );

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
}
