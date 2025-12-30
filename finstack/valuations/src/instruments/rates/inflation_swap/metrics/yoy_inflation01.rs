//! Inflation01 calculator for YoY inflation swaps.
//!
//! Computes inflation sensitivity using finite differences on the inflation curve.

use crate::instruments::inflation_swap::YoYInflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::BumpSpec;
use finstack_core::HashMap;
use finstack_core::Result;

/// Standard inflation curve bump: 1bp (0.0001).
const INFLATION_BUMP_BP: f64 = 0.0001;

/// Inflation01 calculator for YoY inflation swaps.
pub struct YoYInflation01Calculator;

impl MetricCalculator for YoYInflation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap: &YoYInflationSwap = context.instrument_as()?;
        let as_of = context.as_of;

        let bump_spec = BumpSpec::inflation_shift_pct(INFLATION_BUMP_BP * 100.0);
        let mut bumps = HashMap::default();
        bumps.insert(swap.inflation_index_id.clone(), bump_spec);

        let curves_up = context.curves.as_ref().bump(bumps)?;
        let pv_up = swap.npv(&curves_up, as_of)?.amount();

        let bump_spec_down = BumpSpec::inflation_shift_pct(-INFLATION_BUMP_BP * 100.0);
        let mut bumps_down = HashMap::default();
        bumps_down.insert(swap.inflation_index_id.clone(), bump_spec_down);

        let curves_down = context.curves.as_ref().bump(bumps_down)?;
        let pv_down = swap.npv(&curves_down, as_of)?.amount();

        Ok((pv_up - pv_down) / (2.0 * INFLATION_BUMP_BP))
    }
}
