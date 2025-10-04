//! CMBS-specific metric calculators.
//!
//! Implements market-standard metrics for Commercial Mortgage-Backed Securities:
//! - DSCR (Debt Service Coverage Ratio)
//! - WAL TTV (Weighted Average Loan-to-Value)
//! - NCF (Net Cash Flow from properties)
//! - Credit Enhancement
//! - Expected Loss
//! - WAL with prepayments

mod dscr;
mod ltv;

pub use dscr::CmbsDscrCalculator;
pub use ltv::CmbsLtvCalculator;

use crate::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;

/// Register all CMBS metrics
pub fn register_cmbs_metrics(registry: &mut MetricRegistry) {
    // DSCR - Debt Service Coverage Ratio
    registry.register_metric(
        MetricId::custom("cmbs_dscr"),
        Arc::new(CmbsDscrCalculator),
        &["CMBS"],
    );

    // WALTV - Weighted Average LTV
    registry.register_metric(
        MetricId::custom("cmbs_waltv"),
        Arc::new(CmbsLtvCalculator),
        &["CMBS"],
    );

    // WAL with commercial prepayment patterns
    registry.register_metric(
        MetricId::custom("cmbs_wal"),
        Arc::new(CmbsWalCalculator),
        &["CMBS"],
    );

    // Expected Default Rate
    registry.register_metric(
        MetricId::custom("cmbs_default_rate"),
        Arc::new(CmbsDefaultRateCalculator),
        &["CMBS"],
    );

    // Credit Enhancement
    registry.register_metric(
        MetricId::custom("cmbs_ce_level"),
        Arc::new(CmbsCreditEnhancementCalculator),
        &["CMBS"],
    );
}

/// WAL Calculator for CMBS
struct CmbsWalCalculator;

impl crate::metrics::MetricCalculator for CmbsWalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::cmbs::Cmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        Ok(cmbs.pool.weighted_avg_life(context.as_of))
    }
}

/// Default Rate Calculator
struct CmbsDefaultRateCalculator;

impl crate::metrics::MetricCalculator for CmbsDefaultRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::cmbs::Cmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Return current CDR assumption
        Ok(cmbs.cdr_annual.unwrap_or(0.5)) // 0.5% CDR default for commercial
    }
}

/// Credit Enhancement Calculator
struct CmbsCreditEnhancementCalculator;

impl crate::metrics::MetricCalculator for CmbsCreditEnhancementCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let cmbs = context
            .instrument
            .as_any()
            .downcast_ref::<crate::instruments::cmbs::Cmbs>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        if let Some(senior_tranche) = cmbs.tranches.tranches.first() {
            let subordination = cmbs.tranches.subordination_amount(senior_tranche.id.as_str());
            let pool_balance = cmbs.pool.total_balance();

            if pool_balance.amount() > 0.0 {
                Ok(subordination.amount() / pool_balance.amount() * 100.0)
            } else {
                Ok(0.0)
            }
        } else {
            Ok(0.0)
        }
    }
}

