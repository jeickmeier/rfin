//! Vanna calculator for equity options.
//!
//! Computes vanna (∂²V/∂S∂σ) using finite differences.
//! Vanna measures how delta changes with volatility.
//!
//! Vanna ≈ (Delta(σ+h) - Delta(σ-h)) / (2h)
//!
//! Where Delta(σ) is computed by bumping both spot and vol.

use crate::instruments::equity_option::EquityOption;
use crate::metrics::finite_difference::{bump_scalar_price, bump_sizes};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vanna calculator for equity options.
pub struct VannaCalculator;

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
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

        // Get current volatility surface
        let vol_surface = context.curves.surface_ref(option.vol_surface_id.as_str())?;

        // Compute delta at vol_up: bump both spot and vol, compute delta
        let curves_vol_up = {
            let mut curves = context.curves.as_ref().clone();
            let scale_factor = 1.0 + vol_bump;
            use finstack_core::types::CurveId;
            use std::sync::Arc;
            let bumped_surface = vol_surface.scaled(scale_factor);
            curves.surfaces.insert(
                CurveId::from(option.vol_surface_id.as_str()),
                Arc::new(bumped_surface),
            );
            curves
        };

        // Delta at vol_up: (PV(S+h, σ+h) - PV(S-h, σ+h)) / (2h_S)
        let curves_up_vol_up =
            bump_scalar_price(&curves_vol_up, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up_vol_up = option.npv(&curves_up_vol_up, as_of)?.amount();
        let curves_down_vol_up =
            bump_scalar_price(&curves_vol_up, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down_vol_up = option.npv(&curves_down_vol_up, as_of)?.amount();
        let delta_vol_up = (pv_up_vol_up - pv_down_vol_up) / (2.0 * spot_bump);

        // Compute delta at vol_down
        let curves_vol_down = {
            let mut curves = context.curves.as_ref().clone();
            let scale_factor = 1.0 - vol_bump;
            use finstack_core::types::CurveId;
            use std::sync::Arc;
            let bumped_surface = vol_surface.scaled(scale_factor);
            curves.surfaces.insert(
                CurveId::from(option.vol_surface_id.as_str()),
                Arc::new(bumped_surface),
            );
            curves
        };

        // Delta at vol_down: (PV(S+h, σ-h) - PV(S-h, σ-h)) / (2h_S)
        let curves_up_vol_down =
            bump_scalar_price(&curves_vol_down, &option.spot_id, bump_sizes::SPOT)?;
        let pv_up_vol_down = option.npv(&curves_up_vol_down, as_of)?.amount();
        let curves_down_vol_down =
            bump_scalar_price(&curves_vol_down, &option.spot_id, -bump_sizes::SPOT)?;
        let pv_down_vol_down = option.npv(&curves_down_vol_down, as_of)?.amount();
        let delta_vol_down = (pv_up_vol_down - pv_down_vol_down) / (2.0 * spot_bump);

        // Vanna = (Delta(σ+h) - Delta(σ-h)) / (2h_σ)
        // Note: vol_bump is already in absolute terms (0.01), so we need to convert to vol units
        // Since we're bumping by 1%, the denominator is just 2 * vol_bump
        let vanna = (delta_vol_up - delta_vol_down) / (2.0 * vol_bump);

        Ok(vanna)
    }
}
