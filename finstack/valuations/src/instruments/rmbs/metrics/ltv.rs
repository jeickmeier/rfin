//! LTV and FICO calculators for RMBS

use crate::metrics::MetricContext;

/// RMBS Weighted Average LTV calculator
pub struct RmbsLtvCalculator;

impl crate::metrics::MetricCalculator for RmbsLtvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::rmbs::Rmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use credit factors LTV or calculate from pool
        if let Some(ltv) = rmbs.credit_factors.ltv {
            Ok(ltv * 100.0)
        } else {
            Ok(80.0) // Default assumption
        }
    }
}

/// RMBS Weighted Average FICO calculator
pub struct RmbsFicoCalculator;

impl crate::metrics::MetricCalculator for RmbsFicoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::rmbs::Rmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use credit factors credit score or default
        if let Some(fico) = rmbs.credit_factors.credit_score {
            Ok(fico as f64)
        } else {
            Ok(720.0) // Default assumption
        }
    }
}
