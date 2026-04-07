//! Convexity adjustment risk calculator for CMS options.
//!
//! Computes the value of the convexity adjustment by comparing the full PV
//! (with convexity) against the linear PV (without convexity).
//!
//! Risk = PV(full) - PV(linear)
//!
//! This represents the dollar value attributed to the convexity adjustment.

use crate::instruments::rates::cms_option::pricer::CmsOptionPricer;
use crate::instruments::rates::cms_option::types::CmsOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Convexity adjustment risk calculator for CMS options.
pub(crate) struct ConvexityAdjustmentRiskCalculator;

impl MetricCalculator for ConvexityAdjustmentRiskCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CmsOption = context.instrument_as()?;
        let as_of = context.as_of;
        let base_pv = context.base_value.amount();

        // Reprice with zero convexity
        let pricer = CmsOptionPricer::new();
        let linear_pv = pricer
            .price_internal_with_convexity(
                option,
                &context.curves,
                as_of,
                0.0, // No convexity
            )?
            .amount();

        // Risk is the difference
        Ok(base_pv - linear_pv)
    }
}
