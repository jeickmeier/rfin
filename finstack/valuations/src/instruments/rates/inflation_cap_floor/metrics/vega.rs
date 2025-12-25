//! Vega calculator for inflation cap/floor options.
//!
//! Computes vega using finite differences: bump the inflation vol surface,
//! reprice, and compute (PV_vol_up - PV_base) / bump_size.

use crate::instruments::inflation_cap_floor::InflationCapFloor;
use crate::metrics::bump_sizes;
use crate::metrics::scale_surface;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vega calculator for inflation cap/floor options.
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InflationCapFloor = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        if as_of >= option.end_date {
            return Ok(0.0);
        }

        let curves_bumped = scale_surface(
            &context.curves,
            option.vol_surface_id.as_str(),
            1.0 + bump_sizes::VOLATILITY,
        )?;

        let pv_bumped = option.npv(&curves_bumped, as_of)?.amount();
        Ok((pv_bumped - base_pv) / bump_sizes::VOLATILITY)
    }
}
