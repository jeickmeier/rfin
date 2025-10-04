//! RMBS-specific metric calculators.
//!
//! Implements market-standard metrics for Residential Mortgage-Backed Securities:
//! - PSA Speed (Prepayment Speed Assumption)
//! - CPR (Conditional Prepayment Rate)
//! - CDR (Conditional Default Rate)
//! - Severity Rate (Loss Severity)
//! - WAL (Weighted Average Life) with prepayments
//! - WALTV (Weighted Average LTV)
//! - WAFICO (Weighted Average FICO)
//! - Credit Enhancement Levels
//! - Expected Loss

mod cpr;
mod ltv;
mod wal;

pub use cpr::RmbsCprCalculator;
pub use ltv::{RmbsLtvCalculator, RmbsFicoCalculator};
pub use wal::RmbsWalCalculator;

use crate::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;

/// Register all RMBS metrics
pub fn register_rmbs_metrics(registry: &mut MetricRegistry) {
    // CPR - Conditional Prepayment Rate
    registry.register_metric(
        MetricId::custom("rmbs_cpr"),
        Arc::new(RmbsCprCalculator),
        &["RMBS"],
    );

    // CDR - Conditional Default Rate
    registry.register_metric(
        MetricId::custom("rmbs_cdr"),
        Arc::new(RmbsCdrCalculator),
        &["RMBS"],
    );

    // Severity Rate
    registry.register_metric(
        MetricId::custom("rmbs_severity"),
        Arc::new(RmbsSeverityCalculator),
        &["RMBS"],
    );

    // WAL with prepayments
    registry.register_metric(
        MetricId::custom("rmbs_wal"),
        Arc::new(RmbsWalCalculator),
        &["RMBS"],
    );

    // WALTV - Weighted Average LTV
    registry.register_metric(
        MetricId::custom("rmbs_waltv"),
        Arc::new(RmbsLtvCalculator),
        &["RMBS"],
    );

    // WAFICO - Weighted Average FICO
    registry.register_metric(
        MetricId::custom("rmbs_wafico"),
        Arc::new(RmbsFicoCalculator),
        &["RMBS"],
    );

    // Expected Loss
    registry.register_metric(
        MetricId::custom("rmbs_expected_loss"),
        Arc::new(RmbsExpectedLossCalculator),
        &["RMBS"],
    );
}

/// CDR Calculator
struct RmbsCdrCalculator;

impl crate::metrics::MetricCalculator for RmbsCdrCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::rmbs::Rmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Return current CDR assumption based on SDA speed
        // At peak (month 30), SDA 100% gives 0.6% CDR
        Ok(rmbs.sda_speed * 0.6)
    }
}

/// Severity Rate Calculator
struct RmbsSeverityCalculator;

impl crate::metrics::MetricCalculator for RmbsSeverityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::rmbs::Rmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Calculate 1 - recovery rate
        if rmbs.pool.cumulative_defaults.amount() > 0.0 {
            let recovery_rate = rmbs.pool.cumulative_recoveries.amount() 
                / rmbs.pool.cumulative_defaults.amount();
            Ok((1.0 - recovery_rate) * 100.0)
        } else {
            // Default assumption for mortgages
            Ok(40.0) // 40% severity
        }
    }
}

/// Expected Loss Calculator
struct RmbsExpectedLossCalculator;

impl crate::metrics::MetricCalculator for RmbsExpectedLossCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let rmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::rmbs::Rmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Expected Loss = CDR * Severity
        let cdr = rmbs.sda_speed * 0.6 / 100.0; // Convert to decimal
        let severity = 0.40; // 40% default severity for mortgages

        Ok(cdr * severity * 100.0)
    }
}

