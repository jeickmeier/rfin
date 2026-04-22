//! Vega calculator for CMS options.
//!
//! Computes vega (swaption volatility sensitivity) using finite differences:
//! bump volatility surface, reprice, and compute (PV_vol_up - PV_base) / bump_size.
//! Vega is per 1% volatility move.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::cms_option::CmsOption;
use crate::metrics::bump_sizes;
use crate::metrics::bump_surface_vol_absolute;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vega calculator for CMS options.
pub(crate) struct VegaCalculator;

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
            finstack_core::dates::DayCountContext::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Bump volatility surface by an absolute vol amount (vol points).
        let curves_bumped = bump_surface_vol_absolute(
            &context.curves,
            option.vol_surface_id.as_str(),
            bump_sizes::VOLATILITY,
        )?;

        // Reprice with bumped vol
        let pv_bumped = option.value(&curves_bumped, as_of)?.amount();

        // Vega = (PV(σ+Δσ) - PV(σ)) / Δσ  (Δσ in absolute vol units).
        let vega = (pv_bumped - base_pv) / bump_sizes::VOLATILITY;

        Ok(vega)
    }
}
