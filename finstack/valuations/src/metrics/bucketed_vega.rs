//! Reusable helpers for bucketed Vega (volatility sensitivity) using point-wise bumps.
//!
//! Provides generic functions to compute bucketed Vega for instruments that
//! depend on volatility surfaces. Results are stored into `MetricContext` via
//! structured series using stable composite keys.
//!
//! # Bucketing Strategy
//!
//! BucketedVega bumps individual volatility points on the surface by tenor/strike.
//! The bump is typically 1% relative (e.g., if vol is 20%, bump to 20.2%).
//! Results can be stored as:
//! - 1D series (if bucketing by expiry or strike only)
//! - 2D matrix (if bucketing by expiry × strike)

use crate::metrics::{MetricContext, MetricId};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;

/// Standard volatility bump: 1% relative (0.01)
pub const VOL_BUMP_PCT: f64 = 0.01;

/// Standard expiry buckets in years for equity options.
/// Example: [1m, 3m, 6m, 1y, 2y, 3y, 5y]
pub fn standard_equity_expiry_buckets() -> Vec<f64> {
    vec![
        1.0 / 12.0, // 1m
        3.0 / 12.0, // 3m
        6.0 / 12.0, // 6m
        1.0,        // 1y
        2.0,        // 2y
        3.0,        // 3y
        5.0,        // 5y
    ]
}

/// Standard strike buckets (relative to spot) for equity options.
/// Example: [0.5, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5] of spot
pub fn standard_strike_ratios() -> Vec<f64> {
    vec![0.5, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5]
}

/// Compute bucketed Vega by bumping individual vol surface points.
///
/// For each (expiry, strike) pair in `buckets`), this function:
/// 1. Bumps the vol at that specific point by `bump_pct`
/// 2. Creates a new MarketContext with the bumped surface
/// 3. Reprices the instrument
/// 4. Computes Vega = (PV_bumped - PV_base) / bump_pct
///
/// Results are stored as a 2D matrix with rows=expiries, cols=strikes.
///
/// # Arguments
/// * `context` - Metric context with base PV already computed
/// * `vol_id` - ID of the volatility surface
/// * `expiries` - Expiry times in years to bucket
/// * `strikes` - Strike prices to bucket
/// * `spot_price` - Current spot price (for strike ratio computation if needed)
/// * `bump_pct` - Relative bump size (default 0.01 for 1%)
/// * `revalue_with_context` - Function to reprice the instrument
///
/// # Returns
/// Total Vega (sum of all bucketed values)
pub fn compute_bucketed_vega_matrix<I, J, RevalFn>(
    context: &mut MetricContext,
    vol_id: &CurveId,
    expiries: I,
    strikes: J,
    spot_price: Option<f64>,
    bump_pct: f64,
    mut revalue_with_context: RevalFn,
) -> finstack_core::Result<f64>
where
    I: IntoIterator<Item = f64>,
    J: IntoIterator<Item = f64> + Clone,
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let base_ctx = context.curves.as_ref();
    let vol_surface = base_ctx.surface(vol_id.as_str())?; // Returns Arc<VolSurface>

    let expiries_vec: Vec<f64> = expiries.into_iter().collect();
    let strikes_vec: Vec<f64> = strikes.clone().into_iter().collect();

    let mut matrix = Vec::new();
    let mut total_vega = 0.0;

    for &expiry in &expiries_vec {
        let mut row = Vec::new();
        for &strike in &strikes_vec {
            // Resolve strike (use absolute if > 1.0, otherwise treat as ratio of spot)
            let resolved_strike = if let Some(spot) = spot_price {
                if strike < 100.0 {
                    // Assume strike is a ratio
                    strike * spot
                } else {
                    strike
                }
            } else {
                strike
            };

            // Bump vol at this specific (expiry, strike) point using VolSurface::bump_point()
            let bumped_surface = vol_surface.bump_point(expiry, resolved_strike, bump_pct)?;
            let temp_ctx = base_ctx.clone().insert_surface(bumped_surface);
            let pv_bumped = revalue_with_context(&temp_ctx)?;

            // Vega = (PV_bumped - PV_base) / bump_pct
            // Result is per 1% vol move
            let vega = (pv_bumped.amount() - base_pv.amount()) / bump_pct;
            row.push(vega);
            total_vega += vega;
        }
        matrix.push(row);
    }

    // Store as 2D matrix
    let row_labels: Vec<String> = expiries_vec
        .iter()
        .map(|&t| {
            if t < 1.0 {
                format!("{:.0}m", (t * 12.0).round())
            } else {
                format!("{:.0}y", t)
            }
        })
        .collect();
    let col_labels: Vec<String> = strikes_vec.iter().map(|&k| format!("{:.2}", k)).collect();

    let _ = context.store_matrix2d(
        MetricId::custom("bucketed_vega"),
        row_labels,
        col_labels,
        matrix,
    );

    Ok(total_vega)
}
