//! Volga calculator for FX options.
//!
//! Computes volga (∂²V/∂σ²) using finite differences.
//! Volga measures how vega changes with volatility.
//!
//! Uses two-point finite difference on Vega:
//! Volga = [Vega(σ+Δσ) - Vega(σ-Δσ)] / (2 * Δσ) * 0.01
//! (Scaled to be change in Vega per 1% volatility change)

use crate::instruments::fx_option::FxOption;
use crate::metrics::bump_sizes;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Volga calculator for FX options.
pub struct VolgaCalculator;

impl MetricCalculator for VolgaCalculator {
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

        if vol_bump_size.abs() < 1e-9 {
            return Ok(0.0);
        }

        // Two-point finite difference for Volga = ∂Vega/∂σ * 0.01

        // 1. Vega at (σ+Δσ)
        let curves_vol_up = {
            let vol_bumped = vol_surface.bump_point(t, option.strike, vol_bump_pct)?;
            context.curves.as_ref().clone().insert_surface(vol_bumped)
        };
        let vega_up = option.compute_greeks(&curves_vol_up, as_of)?.vega;

        // 2. Vega at (σ-Δσ)
        let curves_vol_down = {
            let vol_bumped = vol_surface.bump_point(t, option.strike, -vol_bump_pct)?;
            context.curves.as_ref().clone().insert_surface(vol_bumped)
        };

        let vega_down = option.compute_greeks(&curves_vol_down, as_of)?.vega;

        // Volga = (Vega(σ+) - Vega(σ-)) / (2 * Δσ) * 0.01
        let volga = (vega_up - vega_down) / (2.0 * vol_bump_size) * 0.01;

        Ok(volga)
    }
}
