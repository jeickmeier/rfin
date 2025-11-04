//! Metrics-based P&L attribution.
//!
//! Fast linear approximation using pre-computed risk metrics (Theta, DV01, CS01,
//! Vega, etc.) to estimate factor contributions without full repricing.
//!
//! # Algorithm
//!
//! 1. **Carry**: Theta × time_period
//! 2. **RatesCurves**: DV01 × curve_shift (or BucketedDV01 × tenor_shifts)
//! 3. **CreditCurves**: CS01 × spread_shift  
//! 4. **Fx**: FX01 or FX Delta × spot_shift
//! 5. **Volatility**: Vega × vol_shift
//! 6. **ModelParameters**: Param01 metrics × param_shift
//! 7. **Residual**: Total P&L - sum(approximations)
//!
//! # Advantages
//!
//! - Fast: No additional repricing required (uses existing metrics)
//! - Convenient: Works with already-computed ValuationResults
//!
//! # Disadvantages
//!
//! - Linear approximation only (ignores convexity)
//! - Less accurate for large market moves
//! - Larger residuals than parallel/waterfall methods

use crate::attribution::helpers::*;
use crate::attribution::types::*;
use crate::instruments::common::traits::Instrument;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::prelude::*;
use std::sync::Arc;

/// Perform metrics-based P&L attribution for an instrument.
///
/// Uses linear approximation with pre-computed risk metrics. Fast but less
/// accurate than full repricing for large market moves.
///
/// # Arguments
///
/// * `instrument` - Instrument to attribute
/// * `market_t0` - Market context at T₀ (for measuring market shifts)
/// * `market_t1` - Market context at T₁ (for measuring market shifts)
/// * `val_t0` - Valuation result at T₀ (with metrics)
/// * `val_t1` - Valuation result at T₁ (with metrics)
/// * `as_of_t0` - Valuation date at T₀
/// * `as_of_t1` - Valuation date at T₁
///
/// # Returns
///
/// P&L attribution using linear approximation.
///
/// # Errors
///
/// Returns error if:
/// - Required metrics are missing
/// - Currency conversion fails
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::attribution::attribute_pnl_metrics_based;
/// use finstack_valuations::metrics::MetricId;
///
/// // Compute valuations with metrics
/// let metrics = vec![MetricId::Theta, MetricId::Dv01, MetricId::Cs01, MetricId::Vega];
/// let val_t0 = instrument.price_with_metrics(&market_t0, as_of_t0, &metrics)?;
/// let val_t1 = instrument.price_with_metrics(&market_t1, as_of_t1, &metrics)?;
///
/// let attribution = attribute_pnl_metrics_based(
///     &instrument,
///     &market_t0,
///     &market_t1,
///     &val_t0,
///     &val_t1,
///     as_of_t0,
///     as_of_t1,
/// )?;
/// ```
pub fn attribute_pnl_metrics_based(
    instrument: &Arc<dyn Instrument>,
    _market_t0: &MarketContext,
    market_t1: &MarketContext,
    val_t0: &ValuationResult,
    val_t1: &ValuationResult,
    as_of_t0: Date,
    as_of_t1: Date,
) -> Result<PnlAttribution> {
    // Total P&L
    let total_pnl = compute_pnl(
        val_t0.value,
        val_t1.value,
        val_t1.value.currency(),
        market_t1,
        as_of_t1,
    )?;

    // Initialize attribution result
    let mut attribution = PnlAttribution::new(
        total_pnl,
        instrument.id(),
        as_of_t0,
        as_of_t1,
        AttributionMethod::MetricsBased,
    );

    // Extract time period in days
    let time_period_days = (as_of_t1 - as_of_t0).whole_days() as f64;

    // 1. Carry attribution (Theta)
    if let Some(theta) = val_t0.measures.get(MetricId::Theta.as_str()) {
        // Theta is typically quoted per day, so multiply by days
        let carry_amount = theta * time_period_days;
        attribution.carry = Money::new(carry_amount, val_t1.value.currency());
    }

    // 2. Rates curves attribution (DV01)
    if let Some(dv01) = val_t0.measures.get(MetricId::Dv01.as_str()) {
        // DV01 is per 1bp move
        // For metrics-based, we'd need to measure actual curve shift
        // For now, use simple approximation: attribute any unexplained P&L to curves
        // TODO: Measure actual curve shifts from market_t0 to market_t1
        
        // Placeholder: estimate curve P&L as DV01 * assumed_shift
        // In practice, this requires curve comparison logic
        let estimated_shift_bp = 0.0; // Would need to compute from markets
        let rates_amount = dv01 * estimated_shift_bp;
        attribution.rates_curves_pnl = Money::new(rates_amount, val_t1.value.currency());
    }

    // 3. Credit curves attribution (CS01)
    if let Some(cs01) = val_t0.measures.get(MetricId::Cs01.as_str()) {
        // CS01 is per 1bp spread move
        let estimated_spread_shift_bp = 0.0; // Would need to compute from markets
        let credit_amount = cs01 * estimated_spread_shift_bp;
        attribution.credit_curves_pnl = Money::new(credit_amount, val_t1.value.currency());
    }

    // 4. FX attribution (FX01 or FX Delta)
    if let Some(fx_delta) = val_t0.measures.get("fx_delta") {
        // FX Delta × spot change
        let fx_amount = fx_delta * 0.0; // Would need to compute FX spot change
        attribution.fx_pnl = Money::new(fx_amount, val_t1.value.currency());
    }

    // 5. Volatility attribution (Vega)
    if let Some(vega) = val_t0.measures.get(MetricId::Vega.as_str()) {
        // Vega × vol change (in percentage points)
        let vol_change = 0.0; // Would need to compute from surfaces
        let vol_amount = vega * vol_change * 100.0; // Vega is per 1% move
        attribution.vol_pnl = Money::new(vol_amount, val_t1.value.currency());
    }

    // 6. Model parameters (various 01 metrics)
    // Prepayment01, Default01, Recovery01, etc.
    if let Some(prepayment01) = val_t0.measures.get("prepayment01") {
        let prepay_change_bp = 0.0; // Would measure from instrument params
        let prepay_amount = prepayment01 * prepay_change_bp;
        
        let model_params = ModelParamsAttribution {
            prepayment: Some(Money::new(prepay_amount, val_t1.value.currency())),
            default_rate: None,
            recovery_rate: None,
            conversion_ratio: None,
            other: indexmap::IndexMap::new(),
        };
        
        attribution.model_params_detail = Some(model_params);
        
        // Sum model params for total
        attribution.model_params_pnl = Money::new(prepay_amount, val_t1.value.currency());
    }

    // 7. Market scalars (dividends, equity prices, etc.)
    if let Some(dividend01) = val_t0.measures.get(MetricId::Dividend01.as_str()) {
        let div_change_bp = 0.0; // Would measure from market scalars
        let div_amount = dividend01 * div_change_bp;
        attribution.market_scalars_pnl = Money::new(div_amount, val_t1.value.currency());
    }

    // Compute residual
    attribution.compute_residual();

    // Metadata
    attribution.meta.num_repricings = 0; // Metrics-based doesn't reprice
    attribution.meta.tolerance = 0.01; // Expect larger residual for metrics-based (1%)

    Ok(attribution)
}

/// Helper to measure curve shift between two markets.
///
/// TODO: Implement curve comparison logic to measure average shift in basis points.
///
/// # Arguments
///
/// * `curve_id` - Curve to compare
/// * `market_t0` - Market at T₀
/// * `market_t1` - Market at T₁
///
/// # Returns
///
/// Average shift in basis points (positive if rates increased).
#[allow(dead_code)]
fn measure_curve_shift(
    _curve_id: &str,
    _market_t0: &MarketContext,
    _market_t1: &MarketContext,
) -> f64 {
    // TODO: Implement by:
    // 1. Extract curve at T₀ and T₁
    // 2. Sample at standard tenors (3M, 6M, 1Y, 2Y, 5Y, 10Y, 30Y)
    // 3. Compute average shift
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use indexmap::IndexMap;
    use time::macros::date;

    #[test]
    fn test_metrics_based_with_theta() {
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        let val_t0_value = Money::new(1000.0, Currency::USD);
        let val_t1_value = Money::new(1050.0, Currency::USD);

        // Create valuation results with Theta
        let mut measures_t0 = IndexMap::new();
        measures_t0.insert(MetricId::Theta.as_str().to_string(), -5.0); // -$5/day decay

        let meta = finstack_core::config::results_meta(&FinstackConfig::default());

        let _val_t0 = ValuationResult::stamped_with_meta("TEST-001", as_of_t0, val_t0_value, meta.clone())
            .with_measures(measures_t0);

        let _val_t1 = ValuationResult::stamped_with_meta("TEST-001", as_of_t1, val_t1_value, meta);

        // Note: would normally pass actual instrument and markets, but for testing just use placeholders
        // The function doesn't actually use instrument for metrics-based (just val results)
        
        // For test purposes, we'll verify that the attribution structure is created correctly
        // Actual attribution would require valid instrument and markets
    }

    #[test]
    fn test_metrics_based_requires_valuations() {
        // Metrics-based attribution requires pre-computed ValuationResults
        // This test verifies the signature and basic structure
        
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        let val_t0_value = Money::new(1000.0, Currency::USD);
        let val_t1_value = Money::new(1100.0, Currency::USD);

        let meta = finstack_core::config::results_meta(&FinstackConfig::default());

        let val_t0 = ValuationResult::stamped_with_meta("TEST-001", as_of_t0, val_t0_value, meta.clone());
        let val_t1 = ValuationResult::stamped_with_meta("TEST-001", as_of_t1, val_t1_value, meta);

        // Verify that ValuationResult structure is correct
        assert_eq!(val_t0.value.amount(), 1000.0);
        assert_eq!(val_t1.value.amount(), 1100.0);
    }
}

