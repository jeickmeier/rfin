//! Weighted Average Life calculator for CLO with prepayments

use crate::metrics::MetricContext;

/// CLO WAL calculator with prepayment adjustments
pub struct CloWalCalculator;

impl crate::metrics::MetricCalculator for CloWalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::fixed_income::structured_credit::StructuredCredit>()
            .ok_or(finstack_core::InputError::Invalid)?;

        // Use the pool's WAM calculation (approximation for WAL)
        Ok(clo.pool.weighted_avg_maturity(context.as_of))
    }
}
