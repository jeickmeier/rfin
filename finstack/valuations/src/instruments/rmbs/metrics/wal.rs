//! Weighted Average Life calculator for RMBS with prepayments

use crate::metrics::MetricContext;

/// RMBS WAL calculator with PSA prepayment adjustments
pub struct RmbsWalCalculator;

impl crate::metrics::MetricCalculator for RmbsWalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::rmbs::Rmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use the pool's WAL calculation, adjusted for PSA speed
        let base_wal = rmbs.pool.weighted_avg_life(context.as_of);

        // Higher PSA speeds shorten WAL
        // Simplified adjustment: WAL / (1 + PSA/2)
        let adjusted_wal = base_wal / (1.0 + rmbs.psa_speed / 2.0);

        Ok(adjusted_wal)
    }
}
