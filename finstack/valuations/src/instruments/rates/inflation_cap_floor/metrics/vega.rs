//! Vega calculator for inflation cap/floor options.
//!
//! Computes vega using central finite differences for O(h²) accuracy:
//!
//! ```text
//! Vega = (PV_vol_up - PV_vol_down) / (2 × bump_size)
//! ```
//!
//! This avoids the O(h) bias that one-sided differences can introduce,
//! especially important for curved volatility surfaces.

use crate::instruments::inflation_cap_floor::InflationCapFloor;
use crate::metrics::bump_sizes;
use crate::metrics::bump_surface_vol_absolute;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vega calculator for inflation cap/floor options.
///
/// Uses central differences for improved accuracy on curved vol surfaces.
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InflationCapFloor = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= option.end_date {
            return Ok(0.0);
        }

        // Bump vol surface up
        let curves_up = bump_surface_vol_absolute(
            &context.curves,
            option.vol_surface_id.as_str(),
            bump_sizes::VOLATILITY,
        )?;
        let pv_up = option.npv(&curves_up, as_of)?.amount();

        // Bump vol surface down
        let curves_down = bump_surface_vol_absolute(
            &context.curves,
            option.vol_surface_id.as_str(),
            -bump_sizes::VOLATILITY,
        )?;
        let pv_down = option.npv(&curves_down, as_of)?.amount();

        // Central difference: (PV_up - PV_down) / (2 × bump_size)
        Ok((pv_up - pv_down) / (2.0 * bump_sizes::VOLATILITY))
    }
}
