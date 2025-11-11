//! FX Vega calculator for quanto options.
//!
//! Computes FX vega (FX volatility sensitivity) using finite differences:
//! bump FX volatility surface, reprice, and compute (PV_vol_up - PV_base) / bump_size.
//! FX Vega is per 1% FX volatility move.
//!
//! # Note
//!
//! Only computed if fx_vol_id is provided. Returns 0 if FX volatility
//! surface is not available.

use crate::metrics::finite_difference::bump_sizes;
use crate::instruments::quanto_option::QuantoOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// FX Vega calculator for quanto options.
pub struct FxVegaCalculator;

impl MetricCalculator for FxVegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &QuantoOption = context.instrument_as()?;
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

        // Get FX volatility surface (if provided)
        let fx_vol_id = option.fx_vol_id.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "fx_vol_id not provided for quanto option".to_string(),
            })
        })?;

        let fx_vol_surface = context.curves.surface_ref(fx_vol_id.as_str())?;

        // Bump FX volatility surface by scaling all values (no grid rebuild)
        let mut curves_bumped = context.curves.as_ref().clone();
        let scale_factor = 1.0 + bump_sizes::VOLATILITY;
        use finstack_core::types::CurveId;
        use std::sync::Arc;
        let bumped_surface = fx_vol_surface.scaled(scale_factor);
        curves_bumped
            .surfaces
            .insert(CurveId::from(fx_vol_id.as_str()), Arc::new(bumped_surface));

        // Reprice with bumped FX vol
        let pv_bumped = option.npv(&curves_bumped, as_of)?.amount();

        // FX Vega = (PV_bumped - PV_base) / bump_size (in vol units)
        let fx_vega = (pv_bumped - base_pv) / bump_sizes::VOLATILITY;

        Ok(fx_vega)
    }
}
