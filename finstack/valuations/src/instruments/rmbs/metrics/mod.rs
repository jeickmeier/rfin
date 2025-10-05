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

mod ltv;
mod wal;

pub use ltv::{RmbsFicoCalculator, RmbsLtvCalculator};
pub use wal::RmbsWalCalculator;

use crate::metrics::{MetricContext, MetricRegistry};

/// Register all RMBS metrics
pub fn register_rmbs_metrics(registry: &mut MetricRegistry) {
    use crate::instruments::common::structured_credit::metrics as sc;
    
    crate::register_metrics! {
        registry: registry,
        instrument: "RMBS",
        metrics: [
            // RMBS-specific metrics
            (LossSeverity, RmbsSeverityCalculator),
            (WAL, RmbsWalCalculator),
            (RmbsWaltv, RmbsLtvCalculator),
            (RmbsWafico, RmbsFicoCalculator),
            (ExpectedLoss, RmbsExpectedLossCalculator),
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
            (CPR, sc::CprCalculator),  // Generic CPR handles RMBS
            (CDR, sc::CdrCalculator),  // Generic CDR handles RMBS
            (BucketedDv01, crate::instruments::common::GenericBucketedDv01::<
                crate::instruments::Rmbs,
            >::default()),
        ]
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
            let recovery_rate =
                rmbs.pool.cumulative_recoveries.amount() / rmbs.pool.cumulative_defaults.amount();
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
