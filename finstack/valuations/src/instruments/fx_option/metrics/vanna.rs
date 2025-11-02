//! Vanna calculator for FX options.
//!
//! Computes vanna (∂²V/∂S∂σ) using finite differences.
//! Vanna measures how delta changes with volatility.
//!
//! Uses four-point finite difference:
//! Vanna = [Δ(S+ΔS, σ+Δσ) - Δ(S+ΔS, σ-Δσ) - Δ(S-ΔS, σ+Δσ) + Δ(S-ΔS, σ-Δσ)] / (4 * ΔS * Δσ)

use crate::instruments::common::metrics::finite_difference::bump_sizes;
use crate::instruments::fx_option::FxOption;
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

        let pair = (option.base_currency, option.quote_currency);
        let vol_surface_id = &option.vol_surface_id;

        // Get current spot and vol for bump sizes
        let fx_matrix = context.curves.fx.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_matrix".to_string(),
            })
        })?;

        let spot_query = finstack_core::money::fx::FxQuery::new(pair.0, pair.1, as_of);
        let current_spot = fx_matrix.rate(spot_query)?.rate;

        let vol_surface = context.curves.surface_ref(vol_surface_id.as_str())?;
        let current_vol = vol_surface.value_clamped(t, option.strike);

        // Calculate bump sizes
        let spot_bump_pct = bump_sizes::SPOT;
        let vol_bump_pct = bump_sizes::VOLATILITY;
        let spot_bump_size = current_spot * spot_bump_pct;
        let vol_bump_size = current_vol * vol_bump_pct;

        // Four-point finite difference for cross-derivative
        // Vanna = [Δ(S+ΔS, σ+Δσ) - Δ(S+ΔS, σ-Δσ) - Δ(S-ΔS, σ+Δσ) + Δ(S-ΔS, σ-Δσ)] / (4 * ΔS * Δσ)

        // 1. Delta at (S+ΔS, σ+Δσ)
        let curves_up_up = {
            let fx_bumped = context
                .curves
                .bump_fx_spot(pair.0, pair.1, spot_bump_pct, as_of)?;
            let vol_bumped = vol_surface.bump_point(t, option.strike, vol_bump_pct)?;
            fx_bumped.insert_surface(vol_bumped)
        };
        let delta_up_up = option.compute_greeks(&curves_up_up, as_of)?.delta;

        // 2. Delta at (S+ΔS, σ-Δσ)
        let curves_up_down = {
            let fx_bumped = context
                .curves
                .bump_fx_spot(pair.0, pair.1, spot_bump_pct, as_of)?;
            let vol_bumped = vol_surface.bump_point(t, option.strike, -vol_bump_pct)?;
            fx_bumped.insert_surface(vol_bumped)
        };
        let delta_up_down = option.compute_greeks(&curves_up_down, as_of)?.delta;

        // 3. Delta at (S-ΔS, σ+Δσ)
        let curves_down_up = {
            let fx_bumped = context
                .curves
                .bump_fx_spot(pair.0, pair.1, -spot_bump_pct, as_of)?;
            let vol_bumped = vol_surface.bump_point(t, option.strike, vol_bump_pct)?;
            fx_bumped.insert_surface(vol_bumped)
        };
        let delta_down_up = option.compute_greeks(&curves_down_up, as_of)?.delta;

        // 4. Delta at (S-ΔS, σ-Δσ)
        let curves_down_down = {
            let fx_bumped = context
                .curves
                .bump_fx_spot(pair.0, pair.1, -spot_bump_pct, as_of)?;
            let vol_bumped = vol_surface.bump_point(t, option.strike, -vol_bump_pct)?;
            fx_bumped.insert_surface(vol_bumped)
        };
        let delta_down_down = option.compute_greeks(&curves_down_down, as_of)?.delta;

        // Four-point finite difference
        let vanna = (delta_up_up - delta_up_down - delta_down_up + delta_down_down)
            / (4.0 * spot_bump_size * vol_bump_size);

        Ok(vanna)
    }
}
