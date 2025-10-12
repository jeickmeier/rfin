//! LTV calculator for CMBS

use crate::constants::DECIMAL_TO_PERCENT;
use crate::metrics::MetricContext;

/// CMBS Weighted Average LTV calculator
pub struct CmbsLtvCalculator {
    default_ltv: f64,
}

impl CmbsLtvCalculator {
    /// Create a new CMBS LTV calculator with specified default LTV (as percentage)
    pub fn new(default_ltv: f64) -> Self {
        Self { default_ltv }
    }
}

impl crate::metrics::MetricCalculator for CmbsLtvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use credit factors LTV or default
        if let Some(ltv) = cmbs.credit_factors.ltv {
            Ok(ltv * DECIMAL_TO_PERCENT)
        } else {
            Ok(self.default_ltv)
        }
    }
}
