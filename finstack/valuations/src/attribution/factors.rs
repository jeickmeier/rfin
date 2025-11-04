//! Factor decomposition logic for P&L attribution.
//!
//! Provides functions to freeze/restore specific market factors while
//! manipulating MarketContext for attribution analysis.

use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::dividends::DividendSchedule;
use finstack_core::market_data::scalars::{MarketScalar, ScalarTimeSeries};
use finstack_core::market_data::scalars::inflation_index::InflationIndex;
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
    pub discount_curves: HashMap<CurveId, Arc<DiscountCurve>>,
    pub forward_curves: HashMap<CurveId, Arc<ForwardCurve>>,
}

/// Snapshot of all credit hazard curves from a market context.
#[derive(Clone, Debug)]
pub struct CreditCurvesSnapshot {
    pub hazard_curves: HashMap<CurveId, Arc<HazardCurve>>,
}

/// Snapshot of all inflation curves from a market context.
#[derive(Clone, Debug)]
pub struct InflationCurvesSnapshot {
    pub inflation_curves: HashMap<CurveId, Arc<InflationCurve>>,
}

/// Snapshot of all base correlation curves from a market context.
#[derive(Clone, Debug)]
pub struct CorrelationsSnapshot {
    pub base_correlation_curves: HashMap<CurveId, Arc<BaseCorrelationCurve>>,
}

/// Snapshot of volatility surfaces from a market context.
#[derive(Clone)]
pub struct VolatilitySnapshot {
    pub surfaces: HashMap<CurveId, Arc<VolSurface>>,
}

/// Snapshot of market scalars from a market context.
#[derive(Clone, Debug)]
pub struct ScalarsSnapshot {
    pub prices: HashMap<CurveId, MarketScalar>,
    pub series: HashMap<CurveId, ScalarTimeSeries>,
    pub inflation_indices: HashMap<CurveId, Arc<InflationIndex>>,
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
///
/// Note: This function clones market scalars. In a future version with
/// public accessors, we could avoid some cloning.
pub fn extract_scalars(_market: &MarketContext) -> ScalarsSnapshot {
    // TODO: MarketContext doesn't expose iterators for prices, series, etc.
    // For now, return empty snapshot. This will be filled in when
    // MarketContext exposes the necessary public API.
    ScalarsSnapshot {
        prices: HashMap::new(),
        series: HashMap::new(),
        inflation_indices: HashMap::new(),
        dividends: HashMap::new(),
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
/// New market context with replaced rates curves.
///
/// Note: This creates a new market with only the snapshot curves plus
/// non-rates curves from the original market. A future enhancement could
/// preserve all other curve types.
pub fn restore_rates_curves(
    market: &MarketContext,
    snapshot: &RatesCurvesSnapshot,
) -> MarketContext {
    // Start with a new empty market
    let mut new_market = MarketContext::new();

    // Copy non-rates curves (credit, inflation, correlations)
    for curve_id in market.curve_ids() {
        // Copy hazard curves
        if let Ok(hazard) = market.get_hazard(curve_id) {
            new_market.insert_hazard_mut(hazard);
        }
        // Copy inflation curves
        else if let Ok(inflation) = market.get_inflation(curve_id) {
            new_market.insert_inflation_mut(inflation);
        }
        // Copy base correlation curves
        else if let Ok(base_corr) = market.get_base_correlation(curve_id) {
            new_market.insert_base_correlation_mut(base_corr);
        }
    }

    // Insert snapshot rates curves
    for (_id, curve) in &snapshot.discount_curves {
        new_market.insert_discount_mut(Arc::clone(curve));
    }
    for (_id, curve) in &snapshot.forward_curves {
        new_market.insert_forward_mut(Arc::clone(curve));
    }

    // Copy other market data (FX, surfaces, etc.)
    if let Some(fx) = &market.fx {
        new_market.insert_fx_mut(Arc::clone(fx));
    }
    new_market.surfaces = market.surfaces.clone();

    new_market
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
/// New market context with replaced credit curves.
pub fn restore_credit_curves(
    market: &MarketContext,
    snapshot: &CreditCurvesSnapshot,
) -> MarketContext {
    // Start with a new empty market
    let mut new_market = MarketContext::new();

    // Copy non-credit curves
    for curve_id in market.curve_ids() {
        // Copy discount curves
        if let Ok(discount) = market.get_discount(curve_id) {
            new_market.insert_discount_mut(discount);
        }
        // Copy forward curves
        else if let Ok(forward) = market.get_forward(curve_id) {
            new_market.insert_forward_mut(forward);
        }
        // Copy inflation curves
        else if let Ok(inflation) = market.get_inflation(curve_id) {
            new_market.insert_inflation_mut(inflation);
        }
        // Copy base correlation curves
        else if let Ok(base_corr) = market.get_base_correlation(curve_id) {
            new_market.insert_base_correlation_mut(base_corr);
        }
    }

    // Insert snapshot hazard curves
    for (_id, curve) in &snapshot.hazard_curves {
        new_market.insert_hazard_mut(Arc::clone(curve));
    }

    // Copy other market data
    if let Some(fx) = &market.fx {
        new_market.insert_fx_mut(Arc::clone(fx));
    }
    new_market.surfaces = market.surfaces.clone();

    new_market
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
/// New market context with replaced inflation curves.
pub fn restore_inflation_curves(
    market: &MarketContext,
    snapshot: &InflationCurvesSnapshot,
) -> MarketContext {
    // Start with a new empty market
    let mut new_market = MarketContext::new();

    // Copy non-inflation curves
    for curve_id in market.curve_ids() {
        if let Ok(discount) = market.get_discount(curve_id) {
            new_market.insert_discount_mut(discount);
        } else if let Ok(forward) = market.get_forward(curve_id) {
            new_market.insert_forward_mut(forward);
        } else if let Ok(hazard) = market.get_hazard(curve_id) {
            new_market.insert_hazard_mut(hazard);
        } else if let Ok(base_corr) = market.get_base_correlation(curve_id) {
            new_market.insert_base_correlation_mut(base_corr);
        }
    }

    // Insert snapshot inflation curves
    for (_id, curve) in &snapshot.inflation_curves {
        new_market.insert_inflation_mut(Arc::clone(curve));
    }

    // Copy other market data
    if let Some(fx) = &market.fx {
        new_market.insert_fx_mut(Arc::clone(fx));
    }
    new_market.surfaces = market.surfaces.clone();

    new_market
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
/// New market context with replaced correlation curves.
pub fn restore_correlations(
    market: &MarketContext,
    snapshot: &CorrelationsSnapshot,
) -> MarketContext {
    // Start with a new empty market
    let mut new_market = MarketContext::new();

    // Copy non-correlation curves
    for curve_id in market.curve_ids() {
        if let Ok(discount) = market.get_discount(curve_id) {
            new_market.insert_discount_mut(discount);
        } else if let Ok(forward) = market.get_forward(curve_id) {
            new_market.insert_forward_mut(forward);
        } else if let Ok(hazard) = market.get_hazard(curve_id) {
            new_market.insert_hazard_mut(hazard);
        } else if let Ok(inflation) = market.get_inflation(curve_id) {
            new_market.insert_inflation_mut(inflation);
        }
    }

    // Insert snapshot base correlation curves
    for (_id, curve) in &snapshot.base_correlation_curves {
        new_market.insert_base_correlation_mut(Arc::clone(curve));
    }

    // Copy other market data
    if let Some(fx) = &market.fx {
        new_market.insert_fx_mut(Arc::clone(fx));
    }
    new_market.surfaces = market.surfaces.clone();

    new_market
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
pub fn restore_volatility(
    market: &MarketContext,
    snapshot: &VolatilitySnapshot,
) -> MarketContext {
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
///
/// Note: Currently returns market unchanged since MarketContext doesn't expose
/// public mutators for prices, series, inflation_indices, and dividends fields.
/// TODO: Add public API to MarketContext for these fields.
pub fn restore_scalars(market: &MarketContext, _snapshot: &ScalarsSnapshot) -> MarketContext {
    // TODO: MarketContext fields (prices, series, inflation_indices, dividends)
    // are private and don't have public setters. For now, return market unchanged.
    // This means market scalars attribution will be zero until API is enhanced.
    market.clone()
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
            .unwrap()
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

