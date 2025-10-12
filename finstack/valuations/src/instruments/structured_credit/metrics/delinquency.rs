//! Delinquency and charge-off calculators for ABS

use crate::constants::DECIMAL_TO_PERCENT;
use crate::metrics::MetricContext;

/// ABS Delinquency Rate calculator
pub struct AbsDelinquencyCalculator {
    delinquency_rate: f64,
}

impl AbsDelinquencyCalculator {
    /// Create a new delinquency calculator with specified rate
    pub fn new(delinquency_rate: f64) -> Self {
        Self { delinquency_rate }
    }
}

impl crate::metrics::MetricCalculator for AbsDelinquencyCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let abs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Calculate delinquency rate from pool
        // Using credit factors delinquency_days as proxy
        let delinquent_balance = abs
            .pool
            .assets
            .iter()
            .filter(|a| !a.is_defaulted) // Not yet defaulted
            .map(|a| a.balance.amount())
            .sum::<f64>();

        let total_balance = abs.pool.performing_balance().amount();

        if total_balance > 0.0 {
            Ok(delinquent_balance / total_balance * DECIMAL_TO_PERCENT * self.delinquency_rate)
        } else {
            Ok(0.0)
        }
    }
}

/// ABS Charge-Off Rate calculator
pub struct AbsChargeOffCalculator;

impl crate::metrics::MetricCalculator for AbsChargeOffCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let abs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Calculate charge-off rate
        let total_balance = abs.pool.total_balance();
        if total_balance.amount() > 0.0 {
            Ok(abs.pool.cumulative_defaults.amount() / total_balance.amount() * DECIMAL_TO_PERCENT)
        } else {
            Ok(0.0)
        }
    }
}
