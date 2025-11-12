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
use std::sync::Arc;

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

/// Compute parallel Vega by bumping the entire volatility surface uniformly.
///
/// Returns the Vega as a single scalar value (PV change per 1% vol shift).
/// Does not store bucketed series in the context.
pub fn compute_parallel_vega<RevalFn>(
    context: &mut MetricContext,
    vol_surface_id: &CurveId,
    bump_pct: f64,
    mut revalue_with_context: RevalFn,
) -> finstack_core::Result<f64>
where
    RevalFn: FnMut(&MarketContext) -> finstack_core::Result<Money>,
{
    let base_pv = context.base_value;
    let base_ctx = context.curves.as_ref();
    let vol_surface = base_ctx.surface(vol_surface_id.as_str())?;

    // Parallel bump the entire surface by scaling it
    // bump_pct of 0.01 means 1% increase, so scale factor is (1 + bump_pct)
    let scale_factor = 1.0 + bump_pct;
    let bumped_surface = vol_surface.scaled(scale_factor);
    let temp_ctx = base_ctx.clone().insert_surface(bumped_surface);
    let pv_bumped = revalue_with_context(&temp_ctx)?;

    // Vega = (PV_bumped - PV_base) / bump_pct
    // Result is per 1% vol move
    let vega = (pv_bumped.amount() - base_pv.amount()) / bump_pct;

    Ok(vega)
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
/// * `vol_surface_id` - ID of the volatility surface
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
    vol_surface_id: &CurveId,
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
    let vol_surface = base_ctx.surface(vol_surface_id.as_str())?; // Returns Arc<VolSurface>

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

// ===== Generic Calculators =====

use crate::instruments::common::traits::Instrument;
use crate::metrics::traits::MetricCalculator;
use crate::metrics::ShockMode;
use std::marker::PhantomData;

/// Generic Vega calculator that works for any instrument implementing
/// the Instrument trait with a vol_surface_id.
///
/// Supports both parallel and key-rate (bucketed) shock modes.
pub struct GenericVega<I> {
    mode: ShockMode,
    _phantom: PhantomData<I>,
}

impl<I> GenericVega<I> {
    /// Create a new GenericVega calculator with the specified shock mode.
    pub fn new(mode: ShockMode) -> Self {
        Self {
            mode,
            _phantom: PhantomData,
        }
    }

    /// Create a parallel shock calculator.
    pub fn parallel() -> Self {
        Self::new(ShockMode::Parallel)
    }

    /// Create a key-rate (bucketed) shock calculator.
    pub fn key_rate() -> Self {
        Self::new(ShockMode::KeyRate)
    }
}

impl<I> Default for GenericVega<I> {
    fn default() -> Self {
        Self::key_rate()
    }
}

impl<I> MetricCalculator for GenericVega<I>
where
    I: Instrument + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;

        // Get vol surface ID from instrument (returns Option<CurveId>)
        let vol_surface_id = instrument
            .vol_surface_id()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;
        let reval = move |temp_ctx: &MarketContext| inst_arc.value(temp_ctx, as_of);

        match self.mode {
            ShockMode::Parallel => {
                // Parallel shock: bump entire surface uniformly
                compute_parallel_vega(context, &vol_surface_id, VOL_BUMP_PCT, reval)
            }
            ShockMode::KeyRate => {
                // Key-rate shock: bump individual (expiry, strike) points
                let expiries = standard_equity_expiry_buckets();
                let strikes = standard_strike_ratios();

                // For now, pass None for spot price (compute_bucketed_vega_matrix handles this)
                // TODO: Extract spot price from instrument if it implements HasEquityUnderlying
                let spot_price = None;

                compute_bucketed_vega_matrix(
                    context,
                    &vol_surface_id,
                    expiries,
                    strikes,
                    spot_price,
                    VOL_BUMP_PCT,
                    reval,
                )
            }
        }
    }
}
