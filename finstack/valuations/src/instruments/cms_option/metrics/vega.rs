//! Vega calculator for CMS options.
//!
//! Computes vega (swaption volatility sensitivity) using finite differences:
//! bump volatility surface, reprice, and compute (PV_vol_up - PV_base) / bump_size.
//! Vega is per 1% volatility move.

use crate::instruments::cms_option::CmsOption;
use crate::metrics::bump_sizes;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vega calculator for CMS options.
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CmsOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Check if expired
        let final_date = option.fixing_dates.last().copied().unwrap_or(as_of);
        let t = option.day_count.year_fraction(
            as_of,
            final_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Get volatility surface (if provided)
        let vol_surface_id = match option.vol_surface_id.as_ref() {
            Some(id) => id,
            None => {
                return Err(finstack_core::Error::from(
                    finstack_core::error::InputError::NotFound {
                        id: "vol_surface_id not provided for CMS option".to_string(),
                    },
                ));
            }
        };

        let vol_surface = context.curves.surface_ref(vol_surface_id.as_str())?;

        // Bump volatility surface by scaling all values (no grid rebuild)
        let mut curves_bumped = context.curves.as_ref().clone();
        let scale_factor = 1.0 + bump_sizes::VOLATILITY;
        use finstack_core::types::CurveId;
        use std::sync::Arc;
        let bumped_surface = vol_surface.scaled(scale_factor);
        curves_bumped.surfaces.insert(
            CurveId::from(vol_surface_id.as_str()),
            Arc::new(bumped_surface),
        );

        // Reprice with bumped vol
        let pv_bumped = option.npv(&curves_bumped, as_of)?.amount();

        // Vega = (PV_bumped - PV_base) / bump_size (in vol units)
        let vega = (pv_bumped - base_pv) / bump_sizes::VOLATILITY;

        Ok(vega)
    }
}
