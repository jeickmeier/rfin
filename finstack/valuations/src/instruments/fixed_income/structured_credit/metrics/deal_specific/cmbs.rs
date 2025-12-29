//! CMBS-specific metrics (LTV, DSCR).

use crate::constants::DECIMAL_TO_PERCENT;
use crate::instruments::structured_credit::StructuredCredit;
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
            .downcast_ref::<StructuredCredit>()
            .ok_or(finstack_core::InputError::Invalid)?;

        // Use credit factors LTV or default
        if let Some(ltv) = cmbs.credit_factors.ltv {
            Ok(ltv * DECIMAL_TO_PERCENT)
        } else {
            Ok(self.default_ltv)
        }
    }
}

/// CMBS DSCR calculator
pub struct CmbsDscrCalculator {
    noi_multiplier: f64,
}

impl CmbsDscrCalculator {
    /// Create a new DSCR calculator with specified NOI multiplier
    pub fn new(noi_multiplier: f64) -> Self {
        Self { noi_multiplier }
    }
}

impl crate::metrics::MetricCalculator for CmbsDscrCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cmbs = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
            .ok_or(finstack_core::InputError::Invalid)?;

        // DSCR = Net Operating Income / Debt Service
        // Assume NOI is a multiple of the pool interest
        let pool_interest = cmbs.pool.weighted_avg_coupon() * cmbs.pool.total_balance()?.amount();
        let noi = pool_interest * self.noi_multiplier;

        // Debt service (interest + principal payments)
        let debt_service = pool_interest;

        if debt_service > 0.0 {
            Ok(noi / debt_service)
        } else {
            Ok(f64::INFINITY)
        }
    }
}
