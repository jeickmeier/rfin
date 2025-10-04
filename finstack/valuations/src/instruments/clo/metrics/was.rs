//! Weighted Average Spread calculator for CLO

use crate::metrics::MetricContext;

/// CLO WAS calculator - in basis points
pub struct CloWasCalculator;

impl crate::metrics::MetricCalculator for CloWasCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        let mut weighted_spread = 0.0;
        let mut total_balance = 0.0;

        for asset in &clo.pool.assets {
            let balance = asset.balance.amount();
            // Use asset rate as proxy for spread (in real implementation would have explicit spread field)
            let spread_bps = asset.rate * 10000.0;

            weighted_spread += balance * spread_bps;
            total_balance += balance;
        }

        if total_balance > 0.0 {
            Ok(weighted_spread / total_balance)
        } else {
            Ok(0.0)
        }
    }
}

