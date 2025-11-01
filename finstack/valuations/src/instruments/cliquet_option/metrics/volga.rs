//! Volga calculator for cliquet options.
//!
//! Computes volga (∂²V/∂σ²) using finite differences.

use crate::instruments::cliquet_option::CliquetOption;
use crate::instruments::common::metrics::finite_difference::bump_sizes;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Volga calculator for cliquet options.
pub struct VolgaCalculator;

impl MetricCalculator for VolgaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CliquetOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        let final_date = option.reset_dates.last().copied().unwrap_or(as_of);
        let t = option.day_count.year_fraction(
            as_of,
            final_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let vol_bump = bump_sizes::VOLATILITY;
        let vol_surface = context.curves.surface_ref(option.vol_id.as_str())?;

        let curves_vol_up = {
            let mut curves = context.curves.as_ref().clone();
            let scale_factor = 1.0 + vol_bump;
            let state = vol_surface.to_state();
            let bumped_vols: Vec<f64> = state
                .vols_row_major
                .iter()
                .map(|v| v * scale_factor)
                .collect();
            use finstack_core::market_data::surfaces::vol_surface::VolSurface;
            use finstack_core::types::CurveId;
            use std::sync::Arc;
            let bumped_surface = VolSurface::from_grid(
                option.vol_id.as_str(),
                &state.expiries,
                &state.strikes,
                &bumped_vols,
            )?;
            curves.surfaces.insert(
                CurveId::from(option.vol_id.as_str()),
                Arc::new(bumped_surface),
            );
            curves
        };
        let pv_vol_up = option.npv(&curves_vol_up, as_of)?.amount();

        let curves_vol_down = {
            let mut curves = context.curves.as_ref().clone();
            let scale_factor = 1.0 - vol_bump;
            let state = vol_surface.to_state();
            let bumped_vols: Vec<f64> = state
                .vols_row_major
                .iter()
                .map(|v| v * scale_factor)
                .collect();
            use finstack_core::market_data::surfaces::vol_surface::VolSurface;
            use finstack_core::types::CurveId;
            use std::sync::Arc;
            let bumped_surface = VolSurface::from_grid(
                option.vol_id.as_str(),
                &state.expiries,
                &state.strikes,
                &bumped_vols,
            )?;
            curves.surfaces.insert(
                CurveId::from(option.vol_id.as_str()),
                Arc::new(bumped_surface),
            );
            curves
        };
        let pv_vol_down = option.npv(&curves_vol_down, as_of)?.amount();

        let volga = (pv_vol_up - 2.0 * base_pv + pv_vol_down) / (vol_bump * vol_bump);
        Ok(volga)
    }
}
