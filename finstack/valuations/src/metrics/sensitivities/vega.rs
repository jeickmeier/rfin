//! Vega calculators for volatility sensitivity.
//!
//! Provides parallel and key-rate vega calculators for instruments with volatility surfaces.

use crate::instruments::common::traits::Instrument;
use crate::metrics::traits::MetricCalculator;
use crate::metrics::{MetricContext, MetricId};
use std::marker::PhantomData;
use std::sync::Arc;

/// Standard volatility bump: 1% relative (0.01)
pub const VOL_BUMP_PCT: f64 = 0.01;

/// Bucket selector for key-rate shocks.
///
/// Determines which time points to use when applying key-rate shocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketSelector {
    /// Use standard buckets defined for the asset class.
    /// - IR: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30] years
    /// - Credit: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30] years
    /// - Equity vol: [1m, 3m, 6m, 1y, 2y, 3y, 5y]
    Standard,

    /// Derive buckets from the curve's knot points.
    /// Uses the actual knot times from the discount/hazard curve.
    CurveKnots,

    /// Derive buckets from the volatility surface grid.
    /// Uses the surface's expiry and strike grid points.
    SurfaceGrid,
}

impl Default for BucketSelector {
    fn default() -> Self {
        Self::Standard
    }
}

/// Standard expiry buckets in years for equity options.
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
pub fn standard_strike_ratios() -> Vec<f64> {
    vec![0.5, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5]
}

/// Parallel vega calculator: bumps entire volatility surface uniformly.
pub struct ParallelVega<I> {
    _phantom: PhantomData<I>,
}

impl<I> ParallelVega<I> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<I> Default for ParallelVega<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I> MetricCalculator for ParallelVega<I>
where
    I: Instrument + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;

        let vol_surface_id = instrument
            .vol_surface_id()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        let base_pv = context.base_value;
        let base_ctx = context.curves.as_ref();
        let vol_surface = base_ctx.surface(vol_surface_id.as_str())?;

        // Parallel bump: scale entire surface by (1 + bump_pct)
        let scale_factor = 1.0 + VOL_BUMP_PCT;
        let bumped_surface = vol_surface.scaled(scale_factor);
        let temp_ctx = base_ctx.clone().insert_surface(bumped_surface);

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;
        let pv_bumped = inst_arc.value(&temp_ctx, as_of)?;

        // Vega = (PV_bumped - PV_base) / bump_pct
        let vega = (pv_bumped.amount() - base_pv.amount()) / VOL_BUMP_PCT;

        Ok(vega)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Key-rate vega calculator: bumps individual (expiry, strike) points.
pub struct KeyRateVega<I> {
    expiries: Vec<f64>,
    strikes: Vec<f64>,
    _phantom: PhantomData<I>,
}

impl<I> KeyRateVega<I> {
    /// Create a key-rate vega calculator with custom buckets.
    pub fn new(expiries: Vec<f64>, strikes: Vec<f64>) -> Self {
        Self {
            expiries,
            strikes,
            _phantom: PhantomData,
        }
    }

    /// Create a key-rate vega calculator with standard equity buckets.
    pub fn standard() -> Self {
        Self::new(standard_equity_expiry_buckets(), standard_strike_ratios())
    }
}

impl<I> Default for KeyRateVega<I> {
    fn default() -> Self {
        Self::standard()
    }
}

impl<I> MetricCalculator for KeyRateVega<I>
where
    I: Instrument + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;

        let vol_surface_id = instrument
            .vol_surface_id()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::error::InputError::Invalid))?;

        let base_pv = context.base_value;
        let base_ctx = context.curves.as_ref();
        let vol_surface = base_ctx.surface(vol_surface_id.as_str())?;

        let inst_arc = Arc::clone(&context.instrument);
        let as_of = context.as_of;

        let mut matrix = Vec::new();
        let mut total_vega = 0.0;

        for &expiry in &self.expiries {
            let mut row = Vec::new();
            for &strike in &self.strikes {
                // Bump vol at this specific (expiry, strike) point
                let bumped_surface = vol_surface.bump_point(expiry, strike, VOL_BUMP_PCT)?;
                let temp_ctx = base_ctx.clone().insert_surface(bumped_surface);
                let pv_bumped = inst_arc.value(&temp_ctx, as_of)?;

                // Vega = (PV_bumped - PV_base) / bump_pct
                let vega = (pv_bumped.amount() - base_pv.amount()) / VOL_BUMP_PCT;
                row.push(vega);
                total_vega += vega;
            }
            matrix.push(row);
        }

        // Store as 2D matrix
        let row_labels: Vec<String> = self
            .expiries
            .iter()
            .map(|&t| {
                if t < 1.0 {
                    format!("{:.0}m", (t * 12.0).round())
                } else {
                    format!("{:.0}y", t)
                }
            })
            .collect();
        let col_labels: Vec<String> = self.strikes.iter().map(|&k| format!("{:.2}", k)).collect();

        let _ = context.store_matrix2d(
            MetricId::custom("bucketed_vega"),
            row_labels,
            col_labels,
            matrix,
        );

        Ok(total_vega)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
