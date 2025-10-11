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

use crate::constants::DECIMAL_TO_PERCENT;
use crate::metrics::{MetricContext, MetricRegistry};

/// Register all CMBS metrics
pub fn register_cmbs_metrics(registry: &mut MetricRegistry) {
    use crate::instruments::common::structured_credit::metrics as sc;
    
    crate::register_metrics! {
        registry: registry,
        instrument: "CMBS",
        metrics: [
            // CMBS-specific metrics
            (CmbsDscr, CmbsDscrCalculator),
            (CmbsWaltv, CmbsLtvCalculator),
            (WAL, CmbsWalCalculator),
            (CmbsCreditEnhancement, CmbsCreditEnhancementCalculator),
            // Shared structured credit metrics
            (Accrued, sc::AccruedCalculator),
            (DirtyPrice, sc::DirtyPriceCalculator),
            (CleanPrice, sc::CleanPriceCalculator),
            (DurationMac, sc::MacaulayDurationCalculator),
            (DurationMod, sc::ModifiedDurationCalculator),
            (ZSpread, sc::ZSpreadCalculator),
            (Cs01, sc::Cs01Calculator),
            (SpreadDuration, sc::SpreadDurationCalculator),
            (Ytm, sc::YtmCalculator),
            (WAM, sc::WamCalculator),
            (CPR, sc::CprCalculator),  // Generic CPR handles CMBS
            (CDR, sc::CdrCalculator),  // Generic CDR handles CMBS (replaces CmbsDefaultRate)
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01::<
                crate::instruments::Cmbs,
            >::default()),
        ]
    }
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

        Ok(cmbs.pool.weighted_avg_maturity(context.as_of))
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
            let subordination = cmbs
                .tranches
                .subordination_amount(senior_tranche.id.as_str());
            let pool_balance = cmbs.pool.total_balance();

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
