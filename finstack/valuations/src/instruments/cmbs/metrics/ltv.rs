//! LTV calculator for CMBS

use crate::metrics::MetricContext;

/// CMBS Weighted Average LTV calculator
pub struct CmbsLtvCalculator;

impl crate::metrics::MetricCalculator for CmbsLtvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::cmbs::Cmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use credit factors LTV or default
        if let Some(ltv) = cmbs.credit_factors.ltv {
            Ok(ltv * 100.0)
        } else {
            Ok(65.0) // Default commercial real estate LTV
        }
    }
}

