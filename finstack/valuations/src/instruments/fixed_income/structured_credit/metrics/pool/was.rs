//! Weighted Average Spread calculator for CLO

use crate::metrics::MetricContext;

/// CLO WAS calculator - in basis points
///
/// Market standard: WAS should use the **spread component only**, not the all-in coupon.
/// For floating rate assets: use the spread over the index (e.g., SOFR + 450bps -> 450)
/// For fixed rate assets: fall back to the all-in rate as a proxy
pub struct CloWasCalculator;

impl crate::metrics::MetricCalculator for CloWasCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::fixed_income::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::InputError::Invalid)?;

        Ok(clo.pool.weighted_avg_spread())
    }
}
