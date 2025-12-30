//! Vanna calculator for FX options.
//!
//! Computes vanna (∂²V/∂S∂σ) using finite differences.
//! Vanna measures how delta changes with volatility.
//!
//! Uses two-point finite difference on Delta:
//! Vanna = [Δ(σ+Δσ) - Δ(σ-Δσ)] / (2 * Δσ)

use crate::instruments::fx_option::FxOption;
use crate::metrics::bump_sizes;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vanna calculator for FX options.
pub struct VannaCalculator;

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &FxOption = context.instrument_as()?;
        let as_of = context.as_of;

        // Check if expired
        let t = option
            .day_count
            .year_fraction(
                as_of,
                option.expiry,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        if t <= 0.0 {
            return Ok(0.0);
        }

        let vol_surface_id = &option.vol_surface_id;
        let vol_surface = context.curves.surface(vol_surface_id.as_str())?;
        let current_vol = vol_surface.value_clamped(t, option.strike);

        // Calculate bump sizes
        let vol_bump_pct = bump_sizes::VOLATILITY;
        let vol_bump_size = current_vol * vol_bump_pct;

        // Two-point finite difference for Vanna = ∂Δ/∂σ

        // 1. Delta at (σ+Δσ)
        let curves_vol_up = {
            let vol_bumped = vol_surface.bump_point(t, option.strike, vol_bump_pct)?;
            context.curves.as_ref().clone().insert_surface(vol_bumped)
        };
        let delta_up = option.compute_greeks(&curves_vol_up, as_of)?.delta;

        // 2. Delta at (σ-Δσ)
        let curves_vol_down = {
            let vol_bumped = vol_surface.bump_point(t, option.strike, -vol_bump_pct)?;
            context.curves.as_ref().clone().insert_surface(vol_bumped)
        };
        let delta_down = option.compute_greeks(&curves_vol_down, as_of)?.delta;

        // Vanna = (Δ(σ+) - Δ(σ-)) / (2 * Δσ)
        let vanna = (delta_up - delta_down) / (2.0 * vol_bump_size);

        Ok(vanna)
    }
}
