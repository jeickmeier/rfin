//! Volga calculator for equity options.
//!
//! Computes volga (∂²V/∂σ²) using finite differences.
//! Volga measures how vega changes with volatility.
//!
//! Volga ≈ (Vega(σ+h) - 2*Vega(σ) + Vega(σ-h)) / h²
//!
//! Or equivalently: (PV(σ+h) - 2*PV(σ) + PV(σ-h)) / h²

use crate::instruments::common::metrics::finite_difference::bump_sizes;
use crate::instruments::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Volga calculator for equity options.
pub struct VolgaCalculator;

impl MetricCalculator for VolgaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &EquityOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Check if expired
        let t = option.day_count.year_fraction(
            as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let vol_bump = bump_sizes::VOLATILITY;

        // Get current volatility
        let vol_surface = context.curves.surface_ref(option.vol_id.as_str())?;

        // Bump vol up
        let curves_vol_up = {
            let mut curves = context.curves.as_ref().clone();
            let scale_factor = 1.0 + vol_bump;
            use finstack_core::types::CurveId;
            use std::sync::Arc;
            let bumped_surface = vol_surface.scaled(scale_factor);
            curves.surfaces.insert(
                CurveId::from(option.vol_id.as_str()),
                Arc::new(bumped_surface),
            );
            curves
        };
        let pv_vol_up = option.npv(&curves_vol_up, as_of)?.amount();

        // Bump vol down
        let curves_vol_down = {
            let mut curves = context.curves.as_ref().clone();
            let scale_factor = 1.0 - vol_bump;
            use finstack_core::types::CurveId;
            use std::sync::Arc;
            let bumped_surface = vol_surface.scaled(scale_factor);
            curves.surfaces.insert(
                CurveId::from(option.vol_id.as_str()),
                Arc::new(bumped_surface),
            );
            curves
        };
        let pv_vol_down = option.npv(&curves_vol_down, as_of)?.amount();

        // Volga = (PV(σ+h) - 2*PV(σ) + PV(σ-h)) / h²
        // h is in vol units (0.01 = 1%)
        let volga = (pv_vol_up - 2.0 * base_pv + pv_vol_down) / (vol_bump * vol_bump);

        Ok(volga)
    }
}
