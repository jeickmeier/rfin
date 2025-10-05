//! CLO-specific metric calculators.
//!
//! Implements market-standard metrics for Collateralized Loan Obligations including:
//! - WARF (Weighted Average Rating Factor)
//! - WAS (Weighted Average Spread)
//! - WAC (Weighted Average Coupon)
//! - WAL (Weighted Average Life) with prepayments
//! - OC Ratios by tranche
//! - IC Ratios by tranche
//! - Diversity Score
//! - Expected Default Rate
//! - Expected Recovery Rate
//! - Break-Even Default Rate
//! - Credit Enhancement Levels

mod wal;
mod warf;
mod was;

pub use wal::CloWalCalculator;
pub use warf::CloWarfCalculator;
pub use was::CloWasCalculator;

use crate::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;

/// Register all CLO metrics
pub fn register_clo_metrics(registry: &mut MetricRegistry) {
    // WAL with prepayments
    registry.register_metric(
        MetricId::custom("clo_wal"),
        Arc::new(CloWalCalculator),
        &["CLO"],
    );

    // WARF - Weighted Average Rating Factor
    registry.register_metric(
        MetricId::custom("clo_warf"),
        Arc::new(CloWarfCalculator),
        &["CLO"],
    );

    // WAS - Weighted Average Spread
    registry.register_metric(
        MetricId::custom("clo_was"),
        Arc::new(CloWasCalculator),
        &["CLO"],
    );

    // WAC - Weighted Average Coupon (uses pool method)
    registry.register_metric(
        MetricId::custom("clo_wac"),
        Arc::new(CloWacCalculator),
        &["CLO"],
    );

    // Diversity Score
    registry.register_metric(
        MetricId::custom("clo_diversity"),
        Arc::new(CloDiversityCalculator),
        &["CLO"],
    );

    // OC Ratio (would be per tranche, using CLASS_A as example)
    registry.register_metric(
        MetricId::custom("clo_oc_ratio"),
        Arc::new(CloOcRatioCalculator),
        &["CLO"],
    );

    // IC Ratio
    registry.register_metric(
        MetricId::custom("clo_ic_ratio"),
        Arc::new(CloIcRatioCalculator),
        &["CLO"],
    );

    // Expected Default Rate
    registry.register_metric(
        MetricId::custom("clo_default_rate"),
        Arc::new(CloDefaultRateCalculator),
        &["CLO"],
    );

    // Expected Recovery Rate
    registry.register_metric(
        MetricId::custom("clo_recovery_rate"),
        Arc::new(CloRecoveryRateCalculator),
        &["CLO"],
    );
}

/// WAC Calculator
struct CloWacCalculator;

impl crate::metrics::MetricCalculator for CloWacCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        Ok(clo.pool.weighted_avg_coupon())
    }
}

/// Diversity Score Calculator
struct CloDiversityCalculator;

impl crate::metrics::MetricCalculator for CloDiversityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        Ok(clo.pool.diversity_score())
    }
}

/// OC Ratio Calculator
struct CloOcRatioCalculator;

impl crate::metrics::MetricCalculator for CloOcRatioCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Calculate for senior-most tranche
        if let Some(senior_tranche) = clo.tranches.tranches.first() {
            let pool_balance = clo.pool.performing_balance();
            let senior_balance = clo.tranches.senior_balance(senior_tranche.id.as_str());
            let denominator = senior_tranche.current_balance.checked_add(senior_balance)?;

            if denominator.amount() > 0.0 {
                Ok(pool_balance.amount() / denominator.amount())
            } else {
                Ok(f64::INFINITY)
            }
        } else {
            Ok(1.0)
        }
    }
}

/// IC Ratio Calculator
struct CloIcRatioCalculator;

impl crate::metrics::MetricCalculator for CloIcRatioCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Simplified: pool interest / tranche interest
        let pool_interest = clo.pool.weighted_avg_coupon() * clo.pool.performing_balance().amount();

        if let Some(senior_tranche) = clo.tranches.tranches.first() {
            let tranche_interest = senior_tranche.current_balance.amount()
                * senior_tranche.coupon.current_rate(context.as_of);

            if tranche_interest > 0.0 {
                Ok(pool_interest / tranche_interest)
            } else {
                Ok(f64::INFINITY)
            }
        } else {
            Ok(1.0)
        }
    }
}

/// Default Rate Calculator
struct CloDefaultRateCalculator;

impl crate::metrics::MetricCalculator for CloDefaultRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Calculate cumulative default rate
        let total_balance = clo.pool.total_balance();
        if total_balance.amount() > 0.0 {
            Ok(clo.pool.cumulative_defaults.amount() / total_balance.amount() * 100.0)
        } else {
            Ok(0.0)
        }
    }
}

/// Recovery Rate Calculator
struct CloRecoveryRateCalculator;

impl crate::metrics::MetricCalculator for CloRecoveryRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let clo = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::clo::Clo>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Calculate recovery rate on defaults
        if clo.pool.cumulative_defaults.amount() > 0.0 {
            Ok(
                clo.pool.cumulative_recoveries.amount() / clo.pool.cumulative_defaults.amount()
                    * 100.0,
            )
        } else {
            Ok(0.0)
        }
    }
}
