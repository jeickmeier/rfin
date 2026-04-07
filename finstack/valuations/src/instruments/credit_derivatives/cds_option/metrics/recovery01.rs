//! Recovery01 calculator for CDS Option.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) change in recovery rate.
//!
//! ## Limitation: Frozen Hazard Curve
//!
//! This calculator bumps the recovery rate but does **not** recalibrate the hazard
//! curve. Since `h ≈ S / (1 - R)`, changing R without recalibrating understates the
//! true recovery sensitivity. Professional systems (Bloomberg, QuantLib) recalibrate.
//! This provides a "local" or "partial" recovery sensitivity only.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard recovery rate bump: 1% (0.01)
const RECOVERY_BUMP: f64 = 0.01;

/// Recovery01 calculator for CDS Option.
pub(crate) struct Recovery01Calculator;

impl MetricCalculator for Recovery01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CDSOption = context.instrument_as()?;
        let as_of = context.as_of;
        let _base_pv = context.base_value.amount();

        // Get base recovery rate
        let base_recovery = option.recovery_rate;

        // Create option with bumped recovery (up).
        // Clamp to (0.001, 0.999) to stay within the valid domain required by
        // CDSOption::validate() which requires R in the open interval (0, 1).
        // At R=1.0 the protection leg PV is zero; at R=0.0 synthetic CDS may fail.
        let mut option_up = option.clone();
        option_up.recovery_rate = (base_recovery + RECOVERY_BUMP).clamp(0.001, 0.999);
        let pv_up = option_up.value(&context.curves, as_of)?.amount();

        // Create option with bumped recovery (down)
        let mut option_down = option.clone();
        option_down.recovery_rate = (base_recovery - RECOVERY_BUMP).clamp(0.001, 0.999);
        let pv_down = option_down.value(&context.curves, as_of)?.amount();

        // Recovery01 = (PV_up - PV_down) / (2 * bump_size)
        let recovery01 = (pv_up - pv_down) / (2.0 * RECOVERY_BUMP);

        Ok(recovery01)
    }
}
