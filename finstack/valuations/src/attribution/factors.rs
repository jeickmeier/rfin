//! Factor decomposition logic for P&L attribution.
//!
//! Provides functions to freeze/restore specific market factors while
//! manipulating MarketContext for attribution analysis.

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

/// Create a hybrid market context: T₀ market data with T₁ structure.
///
/// This is the baseline for carry attribution: values T₁ with market frozen at T₀.
///
/// # Arguments
///
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁ (for structure reference)
///
/// # Returns
///
/// New market context with T₀ market data.
pub fn freeze_all_market(market_t0: &MarketContext, _market_t1: &MarketContext) -> MarketContext {
    // For carry attribution, we simply use the T₀ market as-is
    // (pricing at T₁ date with T₀ market isolates time decay)
    market_t0.clone()
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
    fn test_freeze_all_market() {
        let base_date = date!(2025 - 01 - 15);
        let curve_t0 = create_test_discount_curve("USD-OIS", base_date);
        let curve_t1 = create_test_discount_curve("USD-OIS", base_date);

        let market_t0 = MarketContext::new().insert_discount(curve_t0);
        let market_t1 = MarketContext::new().insert_discount(curve_t1);

        let frozen = freeze_all_market(&market_t0, &market_t1);
        assert!(frozen.get_discount("USD-OIS").is_ok());
    }
}
