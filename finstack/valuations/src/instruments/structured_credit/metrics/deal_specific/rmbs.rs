//! RMBS-specific metrics (LTV, FICO, WAL with PSA adjustments).

use crate::instruments::structured_credit::StructuredCredit;
use crate::metrics::MetricContext;

/// RMBS Weighted Average LTV calculator
pub struct RmbsLtvCalculator {
    default_ltv: f64,
}

impl RmbsLtvCalculator {
    /// Create a new RMBS LTV calculator with specified default LTV (as percentage)
    pub fn new(default_ltv: f64) -> Self {
        Self { default_ltv }
    }
}

impl crate::metrics::MetricCalculator for RmbsLtvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use credit factors LTV or calculate from pool
        if let Some(ltv) = rmbs.credit_factors.ltv {
            Ok(ltv * 100.0)
        } else {
            Ok(self.default_ltv)
        }
    }
}

/// RMBS Weighted Average FICO calculator
pub struct RmbsFicoCalculator {
    default_fico: f64,
}

impl RmbsFicoCalculator {
    /// Create a new RMBS FICO calculator with specified default FICO score
    pub fn new(default_fico: f64) -> Self {
        Self { default_fico }
    }
}

impl crate::metrics::MetricCalculator for RmbsFicoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use credit factors credit score or default
        if let Some(fico) = rmbs.credit_factors.credit_score {
            Ok(fico as f64)
        } else {
            Ok(self.default_fico)
        }
    }
}

/// RMBS WAL calculator with PSA prepayment adjustments
pub struct RmbsWalCalculator;

impl crate::metrics::MetricCalculator for RmbsWalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Use the pool's WAM calculation (approximation), adjusted for PSA speed
        let base_wal = rmbs.pool.weighted_avg_maturity(context.as_of);

        // Extract psa_speed from behavior overrides, default to 1.0 (100% PSA)
        let psa_speed = rmbs.behavior_overrides.psa_speed_multiplier.unwrap_or(1.0);

        // Higher PSA speeds shorten WAL
        // Simplified adjustment: WAL / (1 + PSA/2)
        let adjusted_wal = base_wal / (1.0 + psa_speed / 2.0);

        Ok(adjusted_wal)
    }
}
