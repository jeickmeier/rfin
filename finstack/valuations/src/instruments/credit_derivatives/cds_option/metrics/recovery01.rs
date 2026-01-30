//! Recovery01 calculator for CDS Option.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) change in recovery rate.

use crate::instruments::cds_option::CdsOption;
use crate::instruments::common::traits::InstrumentNpvExt;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard recovery rate bump: 1% (0.01)
const RECOVERY_BUMP: f64 = 0.01;

/// Recovery01 calculator for CDS Option.
pub struct Recovery01Calculator;

impl MetricCalculator for Recovery01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CdsOption = context.instrument_as()?;
        let as_of = context.as_of;
        let _base_pv = context.base_value.amount();

        // Get base recovery rate
        let base_recovery = option.recovery_rate;

        // Create option with bumped recovery (up)
        let mut option_up = option.clone();
        option_up.recovery_rate = (base_recovery + RECOVERY_BUMP).clamp(0.0, 1.0);
        let pv_up = option_up.npv(&context.curves, as_of)?.amount();

        // Create option with bumped recovery (down)
        let mut option_down = option.clone();
        option_down.recovery_rate = (base_recovery - RECOVERY_BUMP).clamp(0.0, 1.0);
        let pv_down = option_down.npv(&context.curves, as_of)?.amount();

        // Recovery01 = (PV_up - PV_down) / (2 * bump_size)
        let recovery01 = (pv_up - pv_down) / (2.0 * RECOVERY_BUMP);

        Ok(recovery01)
    }
}
