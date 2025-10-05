//! DSCR (Debt Service Coverage Ratio) calculator for CMBS

use crate::metrics::MetricContext;

/// CMBS DSCR calculator
pub struct CmbsDscrCalculator;

impl crate::metrics::MetricCalculator for CmbsDscrCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::cmbs::Cmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // DSCR = Net Operating Income / Debt Service
        // Simplified: assume NOI is 1.5x the pool interest
        let pool_interest = cmbs.pool.weighted_avg_coupon() * cmbs.pool.total_balance().amount();
        let noi = pool_interest * 1.5;

        // Debt service (interest + principal payments)
        let debt_service = pool_interest;

        if debt_service > 0.0 {
            Ok(noi / debt_service)
        } else {
            Ok(f64::INFINITY)
        }
    }
}
