//! Weighted Average Life calculator for RMBS with prepayments

use crate::instruments::structured_credit::InstrumentSpecificFields;
use crate::metrics::MetricContext;

/// RMBS WAL calculator with PSA prepayment adjustments
pub struct RmbsWalCalculator;

impl crate::metrics::MetricCalculator for RmbsWalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use the pool's WAM calculation (approximation), adjusted for PSA speed
        let base_wal = rmbs.pool.weighted_avg_maturity(context.as_of);

        // Extract psa_speed from specific fields
        let psa_speed = match &rmbs.specific {
            InstrumentSpecificFields::Rmbs { psa_speed, .. } => *psa_speed,
            _ => 1.0, // Default to 100% PSA if not RMBS
        };

        // Higher PSA speeds shorten WAL
        // Simplified adjustment: WAL / (1 + PSA/2)
        let adjusted_wal = base_wal / (1.0 + psa_speed / 2.0);

        Ok(adjusted_wal)
    }
}
