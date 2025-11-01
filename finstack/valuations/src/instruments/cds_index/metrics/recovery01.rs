//! Recovery01 calculator for CDS Index.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) change in recovery rate.

use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard recovery rate bump: 1% (0.01)
const RECOVERY_BUMP: f64 = 0.01;

/// Recovery01 calculator for CDS Index.
pub struct Recovery01Calculator;

impl MetricCalculator for Recovery01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: &CDSIndex = context.instrument_as()?;
        let as_of = context.as_of;
        let _base_pv = context.base_value.amount();

        // Get base recovery rate
        let base_recovery = index.protection.recovery_rate;

        // Create index with bumped recovery (up)
        let mut index_up = index.clone();
        index_up.protection.recovery_rate = (base_recovery + RECOVERY_BUMP).clamp(0.0, 1.0);
        let pv_up = index_up.npv(&context.curves, as_of)?.amount();

        // Create index with bumped recovery (down)
        let mut index_down = index.clone();
        index_down.protection.recovery_rate = (base_recovery - RECOVERY_BUMP).clamp(0.0, 1.0);
        let pv_down = index_down.npv(&context.curves, as_of)?.amount();

        // Recovery01 = (PV_up - PV_down) / (2 * bump_size)
        let recovery01 = (pv_up - pv_down) / (2.0 * RECOVERY_BUMP);

        Ok(recovery01)
    }
}
