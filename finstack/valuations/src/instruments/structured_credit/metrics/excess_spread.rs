//! Excess Spread and Credit Enhancement calculators for ABS

use crate::constants::DECIMAL_TO_PERCENT;
use crate::metrics::MetricContext;

/// ABS Excess Spread calculator
pub struct AbsExcessSpreadCalculator {
    servicing_fee_rate: f64,
}

impl AbsExcessSpreadCalculator {
    /// Create a new excess spread calculator with specified servicing fee rate
    pub fn new(servicing_fee_rate: f64) -> Self {
        Self { servicing_fee_rate }
    }
}

impl crate::metrics::MetricCalculator for AbsExcessSpreadCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let abs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Excess Spread = WAC - WAC of tranches - servicing fees
        let pool_wac = abs.pool.weighted_avg_coupon();

        // Calculate weighted average tranche rate
        let mut tranche_weighted_rate = 0.0;
        let mut tranche_total_balance = 0.0;

        for tranche in &abs.tranches.tranches {
            let rate = tranche.coupon.current_rate(context.as_of);
            tranche_weighted_rate += rate * tranche.current_balance.amount();
            tranche_total_balance += tranche.current_balance.amount();
        }

        let tranche_wac = if tranche_total_balance > 0.0 {
            tranche_weighted_rate / tranche_total_balance
        } else {
            0.0
        };

        let excess_spread = pool_wac - tranche_wac - self.servicing_fee_rate;

        Ok(excess_spread * DECIMAL_TO_PERCENT) // Return as percentage
    }
}

/// ABS Credit Enhancement Level calculator
pub struct AbsCreditEnhancementCalculator;

impl crate::metrics::MetricCalculator for AbsCreditEnhancementCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let abs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Credit Enhancement = Subordination + OC + Excess Spread
        // Simplified: subordination for most senior tranche
        if let Some(senior_tranche) = abs.tranches.tranches.first() {
            let subordination = abs
                .tranches
                .subordination_amount(senior_tranche.id.as_str());
            let pool_balance = abs.pool.total_balance();

            if pool_balance.amount() > 0.0 {
                Ok(subordination.amount() / pool_balance.amount() * DECIMAL_TO_PERCENT)
            } else {
                Ok(0.0)
            }
        } else {
            Ok(0.0)
        }
    }
}
