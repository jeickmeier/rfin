//! Delta calculator for CMS options.
//!
//! Computes delta (sensitivity to the underlying rate) using finite differences.
//! For CMS, this means sensitivity to the forward curve that drives the CMS rate.
//!
//! Delta = PV(forward_curve + 1bp) - PV(base)

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::cms_option::CmsOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::Result;

/// Delta calculator for CMS options.
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CmsOption = context.instrument_as()?;
        let base_pv = context.base_value.amount();

        // Determine which curve drives the forward rate
        let curve_to_bump = &option.forward_curve_id;

        // Bump the relevant curve by 1bp (parallel shift)
        let bump_bp = 1.0;
        let curves_bumped = context.curves.bump([MarketBump::Curve {
            id: curve_to_bump.clone(),
            spec: BumpSpec::parallel_bp(bump_bp),
        }])?;

        // Reprice
        let pv_bumped = option.value(&curves_bumped, context.as_of)?.amount();

        // Delta = Change in PV
        Ok(pv_bumped - base_pv)
    }
}
