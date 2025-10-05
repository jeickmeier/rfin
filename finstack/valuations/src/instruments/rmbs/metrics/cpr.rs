//! CPR (Conditional Prepayment Rate) calculator for RMBS

use crate::metrics::MetricContext;

/// RMBS CPR calculator
pub struct RmbsCprCalculator;

impl crate::metrics::MetricCalculator for RmbsCprCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::rmbs::Rmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Return current CPR assumption based on PSA speed
        // PSA 100% = 6% CPR terminal
        Ok(rmbs.psa_speed * 6.0)
    }
}
