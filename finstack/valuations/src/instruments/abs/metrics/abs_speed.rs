//! ABS Speed calculator for auto loans

use crate::metrics::MetricContext;

/// ABS Speed calculator - monthly absolute prepayment speed
pub struct AbsSpeedCalculator;

impl crate::metrics::MetricCalculator for AbsSpeedCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let abs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::abs::Abs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Return ABS speed if set, otherwise default
        Ok(abs.abs_speed.unwrap_or(1.5)) // 1.5% ABS default
    }
}
