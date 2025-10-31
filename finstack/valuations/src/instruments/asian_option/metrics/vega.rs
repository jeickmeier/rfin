//! Vega calculator for Asian options.
//!
//! Computes vega using finite differences: bump volatility surface,
//! reprice, and compute (PV_vol_up - PV_base) / bump_size.
//! Vega is per 1% volatility move.

use crate::instruments::asian_option::AsianOption;
use crate::instruments::common::metrics::finite_difference::bump_sizes;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vega calculator for Asian options.
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &AsianOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Check if expired
        let t = option
            .day_count
            .year_fraction(as_of, option.expiry, finstack_core::dates::DayCountCtx::default())?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Bump volatility surface by scaling all values
        // Use the bump_vol_surface_parallel helper for cleaner implementation
        use crate::instruments::common::metrics::finite_difference::bump_vol_surface_parallel;
        
        let curves_bumped = bump_vol_surface_parallel(
            context.curves.as_ref(),
            option.vol_id.as_str(),
            bump_sizes::VOLATILITY,
        )?;

        // Reprice with bumped vol
        let pv_bumped = option.npv(&curves_bumped, as_of)?.amount();

        // Vega = (PV_bumped - PV_base) / bump_size (in vol units)
        // Since bump_size is 1% (0.01), vega is per 1% vol move
        let vega = (pv_bumped - base_pv) / bump_sizes::VOLATILITY;

        Ok(vega)
    }
}

