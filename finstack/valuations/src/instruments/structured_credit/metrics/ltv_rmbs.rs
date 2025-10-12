//! LTV and FICO calculators for RMBS

use crate::metrics::MetricContext;

/// RMBS Weighted Average LTV calculator
pub struct RmbsLtvCalculator {
    default_ltv: f64,
}

impl RmbsLtvCalculator {
    /// Create a new RMBS LTV calculator with specified default LTV (as percentage)
    pub fn new(default_ltv: f64) -> Self {
        Self { default_ltv }
    }
}

impl crate::metrics::MetricCalculator for RmbsLtvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use credit factors LTV or calculate from pool
        if let Some(ltv) = rmbs.credit_factors.ltv {
            Ok(ltv * 100.0)
        } else {
            Ok(self.default_ltv)
        }
    }
}

/// RMBS Weighted Average FICO calculator
pub struct RmbsFicoCalculator {
    default_fico: f64,
}

impl RmbsFicoCalculator {
    /// Create a new RMBS FICO calculator with specified default FICO score
    pub fn new(default_fico: f64) -> Self {
        Self { default_fico }
    }
}

impl crate::metrics::MetricCalculator for RmbsFicoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use credit factors credit score or default
        if let Some(fico) = rmbs.credit_factors.credit_score {
            Ok(fico as f64)
        } else {
            Ok(self.default_fico)
        }
    }
}
