//! Vanna calculator for barrier options.
//!
//! Computes vanna (∂²V/∂S∂σ) using finite differences.
//! Vanna measures how delta changes with volatility.
//! Note: Barrier options exhibit discontinuous vanna near barrier levels.

use crate::instruments::barrier_option::BarrierOption;
use crate::instruments::common::metrics::finite_difference::{bump_scalar_price, bump_sizes};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vanna calculator for barrier options.
pub struct VannaCalculator;

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &BarrierOption = context.instrument_as()?;
        let as_of = context.as_of;

        // Check if expired
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Get current spot
        let spot_scalar = context.curves.price(&option.spot_id)?;
        let current_spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };

        let spot_bump = current_spot * bump_sizes::SPOT;
        let vol_bump = bump_sizes::VOLATILITY;
        let vol_surface = context.curves.surface_ref(option.vol_id.as_str())?;

        // Delta at vol_up
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

        let curves_up_vol_up =
            bump_scalar_price(&curves_vol_up, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up_vol_up = option.npv(&curves_up_vol_up, as_of)?.amount();
        let curves_down_vol_up =
            bump_scalar_price(&curves_vol_up, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down_vol_up = option.npv(&curves_down_vol_up, as_of)?.amount();
        let delta_vol_up = (pv_up_vol_up - pv_down_vol_up) / (2.0 * spot_bump);

        // Delta at vol_down
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

        let curves_up_vol_down =
            bump_scalar_price(&curves_vol_down, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up_vol_down = option.npv(&curves_up_vol_down, as_of)?.amount();
        let curves_down_vol_down =
            bump_scalar_price(&curves_vol_down, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down_vol_down = option.npv(&curves_down_vol_down, as_of)?.amount();
        let delta_vol_down = (pv_up_vol_down - pv_down_vol_down) / (2.0 * spot_bump);

        // Vanna = (Delta(σ+h) - Delta(σ-h)) / (2h_σ)
        let vanna = (delta_vol_up - delta_vol_down) / (2.0 * vol_bump);

        Ok(vanna)
    }
}
