//! Vega calculator for quanto options.
//!
//! Computes vega (equity volatility sensitivity) using finite differences:
//! bump equity volatility surface, reprice, and compute (PV_vol_up - PV_base) / bump_size.
//! Vega is per 1% volatility move.

use crate::instruments::common::traits::Instrument;
use crate::instruments::quanto_option::QuantoOption;
use crate::metrics::bump_sizes;
use crate::metrics::bump_surface_vol_absolute;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vega calculator for quanto options (equity volatility sensitivity).
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
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

        // Bump equity volatility surface by an absolute vol amount (vol points).
        let curves_bumped = bump_surface_vol_absolute(
            &context.curves,
            option.vol_surface_id.as_str(),
            bump_sizes::VOLATILITY,
        )?;

        // Reprice with bumped vol
        let pv_bumped = option.value(&curves_bumped, as_of)?.amount();

        // Vega = (PV_bumped - PV_base) / bump_size (in vol units)
        let vega = (pv_bumped - base_pv) / bump_sizes::VOLATILITY;

        Ok(vega)
    }
}
