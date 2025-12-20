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
    // Clone the market to preserve all data, then rebuild with updated curves
    let mut temp_market = MarketContext::new();

    // Copy non-rates curves (credit, inflation, correlations)
    for curve_id in market.curve_ids() {
        // Copy hazard curves
        if let Ok(hazard) = market.get_hazard(curve_id) {
            temp_market = temp_market.insert_hazard(hazard);
        }
        // Copy inflation curves
        else if let Ok(inflation) = market.get_inflation(curve_id) {
            temp_market = temp_market.insert_inflation(inflation);
        }
        // Copy base correlation curves
        else if let Ok(base_corr) = market.get_base_correlation(curve_id) {
            temp_market = temp_market.insert_base_correlation(base_corr);
        }
    }

    // Insert snapshot rates curves
    for (_id, curve) in &snapshot.discount_curves {
        temp_market = temp_market.insert_discount(Arc::clone(curve));
    }
    for (_id, curve) in &snapshot.forward_curves {
        temp_market = temp_market.insert_forward(Arc::clone(curve));
    }

    // Copy other market data (FX, surfaces, scalars) from original market
    if let Some(fx) = &market.fx {
        temp_market.insert_fx_mut(Arc::clone(fx));
    }
    temp_market.surfaces = market.surfaces.clone();

    copy_scalars(market, &mut temp_market);

    temp_market
}

/// Replace credit curves in a market context with curves from a snapshot.
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
    // Clone the market to preserve all data, then rebuild with updated curves
    let mut temp_market = MarketContext::new();

    // Copy non-credit curves
    for curve_id in market.curve_ids() {
        // Copy discount curves
        if let Ok(discount) = market.get_discount(curve_id) {
            temp_market = temp_market.insert_discount(discount);
        }
        // Copy forward curves
        else if let Ok(forward) = market.get_forward(curve_id) {
            temp_market = temp_market.insert_forward(forward);
        }
        // Copy inflation curves
        else if let Ok(inflation) = market.get_inflation(curve_id) {
            temp_market = temp_market.insert_inflation(inflation);
        }
        // Copy base correlation curves
        else if let Ok(base_corr) = market.get_base_correlation(curve_id) {
            temp_market = temp_market.insert_base_correlation(base_corr);
        }
    }

    // Insert snapshot hazard curves
    for (_id, curve) in &snapshot.hazard_curves {
        temp_market = temp_market.insert_hazard(Arc::clone(curve));
    }

    // Copy other market data (FX, surfaces, scalars)
    if let Some(fx) = &market.fx {
        temp_market.insert_fx_mut(Arc::clone(fx));
    }
    temp_market.surfaces = market.surfaces.clone();

    copy_scalars(market, &mut temp_market);

    temp_market
}

/// Replace inflation curves in a market context with curves from a snapshot.
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
    // Clone the market to preserve all data, then rebuild with updated curves
    let mut temp_market = MarketContext::new();

    // Copy non-inflation curves
    for curve_id in market.curve_ids() {
        if let Ok(discount) = market.get_discount(curve_id) {
            temp_market = temp_market.insert_discount(discount);
        } else if let Ok(forward) = market.get_forward(curve_id) {
            temp_market = temp_market.insert_forward(forward);
        } else if let Ok(hazard) = market.get_hazard(curve_id) {
            temp_market = temp_market.insert_hazard(hazard);
        } else if let Ok(base_corr) = market.get_base_correlation(curve_id) {
            temp_market = temp_market.insert_base_correlation(base_corr);
        }
    }

    // Insert snapshot inflation curves
    for (_id, curve) in &snapshot.inflation_curves {
        temp_market = temp_market.insert_inflation(Arc::clone(curve));
    }

    // Copy other market data (FX, surfaces, scalars)
    if let Some(fx) = &market.fx {
        temp_market.insert_fx_mut(Arc::clone(fx));
    }
    temp_market.surfaces = market.surfaces.clone();

    copy_scalars(market, &mut temp_market);

    temp_market
}

/// Replace correlation curves in a market context with curves from a snapshot.
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
    // Clone the market to preserve all data, then rebuild with updated curves
    let mut temp_market = MarketContext::new();

    // Copy non-correlation curves
    for curve_id in market.curve_ids() {
        if let Ok(discount) = market.get_discount(curve_id) {
            temp_market = temp_market.insert_discount(discount);
        } else if let Ok(forward) = market.get_forward(curve_id) {
            temp_market = temp_market.insert_forward(forward);
        } else if let Ok(hazard) = market.get_hazard(curve_id) {
            temp_market = temp_market.insert_hazard(hazard);
        } else if let Ok(inflation) = market.get_inflation(curve_id) {
            temp_market = temp_market.insert_inflation(inflation);
        }
    }

    // Insert snapshot base correlation curves
    for (_id, curve) in &snapshot.base_correlation_curves {
        temp_market = temp_market.insert_base_correlation(Arc::clone(curve));
    }

    // Copy other market data (FX, surfaces, scalars)
    if let Some(fx) = &market.fx {
        temp_market.insert_fx_mut(Arc::clone(fx));
    }
    temp_market.surfaces = market.surfaces.clone();

    copy_scalars(market, &mut temp_market);

    temp_market
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
}
