//! Recovery01 calculator for CDS.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) change in recovery rate.
//!
//! Formula: Recovery01 = (PV(recovery + 1%) - PV(recovery - 1%)) / 2
//!
//! # Note
//!
//! Recovery rate changes affect both the protection leg (LGD = 1 - recovery)
//! and the premium leg (accrued on default settlement). This metric captures
//! the full sensitivity across both legs.

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard recovery rate bump: 1% (0.01)
const RECOVERY_BUMP: f64 = 0.01;

/// Recovery01 calculator for CDS.
pub struct Recovery01Calculator;

impl MetricCalculator for Recovery01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Get base recovery rate
        let base_recovery = cds.protection.recovery_rate;

        // Create CDS with bumped recovery (up)
        let mut cds_up = cds.clone();
        cds_up.protection.recovery_rate = (base_recovery + RECOVERY_BUMP).clamp(0.0, 1.0);
        let pv_up = cds_up.npv(&context.curves, as_of)?.amount();

        // Create CDS with bumped recovery (down)
        let mut cds_down = cds.clone();
        cds_down.protection.recovery_rate = (base_recovery - RECOVERY_BUMP).clamp(0.0, 1.0);
        let pv_down = cds_down.npv(&context.curves, as_of)?.amount();

        // Recovery01 = (PV_up - PV_down) / 2
        // This returns the PV change for a 1% (100bp) recovery move.
        let recovery01 = (pv_up - pv_down) / 2.0;

        Ok(recovery01)
    }
}
