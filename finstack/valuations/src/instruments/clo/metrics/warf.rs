//! Weighted Average Rating Factor calculator for CLO

use crate::metrics::MetricContext;
use crate::instruments::common::structured_credit::rating_factors;

/// CLO WARF calculator - Moody's methodology
pub struct CloWarfCalculator;

impl crate::metrics::MetricCalculator for CloWarfCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        let mut weighted_sum = 0.0;
        let mut total_balance = 0.0;

        for asset in &clo.pool.assets {
            let balance = asset.balance.amount();
            let rating_factor = asset
                .credit_quality
                .map(rating_factors::moodys_warf_factor)
                .unwrap_or(3650.0); // Default to B-/CCC+ equivalent

            weighted_sum += balance * rating_factor;
            total_balance += balance;
        }

        if total_balance > 0.0 {
            Ok(weighted_sum / total_balance)
        } else {
            Ok(0.0)
        }
    }
}

