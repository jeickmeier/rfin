//! Recovery01 calculator for CDS Index.
//!
//! Computes Recovery01 (recovery rate sensitivity) using finite differences.
//! Recovery01 measures the change in PV for a 1% (100bp) absolute change in recovery rate.

use crate::instruments::common::traits::Instrument;
use crate::instruments::credit_derivatives::cds_index::CDSIndex;
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

        let bump = |idx: &CDSIndex, delta: f64| -> CDSIndex {
            let mut bumped = idx.clone();
            if bumped.constituents.is_empty() {
                let base = bumped.protection.recovery_rate;
                bumped.protection.recovery_rate = (base + delta).clamp(0.0, 1.0);
            } else {
                for con in &mut bumped.constituents {
                    let base = con.credit.recovery_rate;
                    con.credit.recovery_rate = (base + delta).clamp(0.0, 1.0);
                }
            }
            bumped
        };

        let index_up = bump(index, RECOVERY_BUMP);
        let pv_up = index_up.value(&context.curves, as_of)?.amount();

        let index_down = bump(index, -RECOVERY_BUMP);
        let pv_down = index_down.value(&context.curves, as_of)?.amount();

        // Recovery01 = PV change for a 1% recovery shift (symmetric bump)
        let recovery01 = (pv_up - pv_down) / 2.0;

        Ok(recovery01)
    }
}
