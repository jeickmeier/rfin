//! Recovery01 calculator for CDS.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) change in recovery rate.
//!
//! ## Methodology
//!
//! Uses central differences when possible, with automatic fallback to one-sided
//! differences at recovery rate boundaries:
//!
//! - **Central difference** (interior): `(PV(R+h) - PV(R-h)) / (2h)`
//! - **Forward difference** (near R=0): `(PV(R+h) - PV(R)) / h`
//! - **Backward difference** (near R=1): `(PV(R) - PV(R-h)) / h`
//!
//! This ensures consistent, unbiased sensitivity estimates even when the base
//! recovery rate is near the valid bounds [0, 1].
//!
//! ## Note
//!
//! Recovery rate changes affect both the protection leg (LGD = 1 - recovery)
//! and the premium leg (accrued on default settlement). This metric captures
//! the full sensitivity across both legs.

use crate::instruments::cds::CreditDefaultSwap;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard recovery rate bump: 1% (0.01)
const RECOVERY_BUMP: f64 = 0.01;

/// Minimum bump size considered valid for finite differences.
/// Below this threshold, we treat the bump as ineffective.
const MIN_EFFECTIVE_BUMP: f64 = 1e-6;

/// Recovery01 calculator for CDS.
pub struct Recovery01Calculator;

impl MetricCalculator for Recovery01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Get base recovery rate
        let base_recovery = cds.protection.recovery_rate;

        // Compute effective bump sizes after clamping to [0, 1]
        let bumped_up = (base_recovery + RECOVERY_BUMP).clamp(0.0, 1.0);
        let bumped_down = (base_recovery - RECOVERY_BUMP).clamp(0.0, 1.0);
        let up_delta = bumped_up - base_recovery;
        let down_delta = base_recovery - bumped_down;

        let can_bump_up = up_delta > MIN_EFFECTIVE_BUMP;
        let can_bump_down = down_delta > MIN_EFFECTIVE_BUMP;

        // Determine which finite difference method to use based on available bumps
        let slope = match (can_bump_up, can_bump_down) {
            (true, true) => {
                // Central difference: most accurate, use when both bumps available
                let mut cds_up = cds.clone();
                cds_up.protection.recovery_rate = bumped_up;
                let pv_up = cds_up.value(&context.curves, as_of)?.amount();

                let mut cds_down = cds.clone();
                cds_down.protection.recovery_rate = bumped_down;
                let pv_down = cds_down.value(&context.curves, as_of)?.amount();

                (pv_up - pv_down) / (up_delta + down_delta)
            }
            (true, false) => {
                // Forward difference: recovery near 0, can only bump up
                let base_pv = cds.value(&context.curves, as_of)?.amount();

                let mut cds_up = cds.clone();
                cds_up.protection.recovery_rate = bumped_up;
                let pv_up = cds_up.value(&context.curves, as_of)?.amount();

                (pv_up - base_pv) / up_delta
            }
            (false, true) => {
                // Backward difference: recovery near 1, can only bump down
                let base_pv = cds.value(&context.curves, as_of)?.amount();

                let mut cds_down = cds.clone();
                cds_down.protection.recovery_rate = bumped_down;
                let pv_down = cds_down.value(&context.curves, as_of)?.amount();

                (base_pv - pv_down) / down_delta
            }
            (false, false) => {
                // Cannot bump in either direction (recovery exactly at bound with zero bump)
                // This is an edge case that shouldn't occur with RECOVERY_BUMP = 0.01
                0.0
            }
        };

        // Return the PV change for a 1% (100bp) recovery move.
        let recovery01 = slope * RECOVERY_BUMP;

        Ok(recovery01)
    }
}
