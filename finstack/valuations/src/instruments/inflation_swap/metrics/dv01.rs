//! DV01 (nominal interest rate sensitivity) metric for `InflationSwap`.

use crate::instruments::inflation_swap::InflationSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;

/// Calculates DV01 (1bp nominal interest rate sensitivity) for inflation swaps.
pub struct InflationSwapDv01Calculator;

impl MetricCalculator for InflationSwapDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let s: &InflationSwap = context.instrument_as()?;

        let disc = context.curves.get_discount_ref(s.disc_id.as_str())?;
        let base = disc.base_date();

        let t_maturity = DayCount::Act365F
            .year_fraction(
                base,
                s.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        // Use net PV for sensitivity sign correctness, matching instrument pv body
        let pv_fixed = s.pv_fixed_leg(&context.curves, context.as_of)?;
        let pv_inflation = s.pv_inflation_leg(&context.curves, context.as_of)?;
        let pv_net = match s.side {
            crate::instruments::inflation_swap::PayReceiveInflation::ReceiveFixed => {
                (pv_fixed - pv_inflation)?.amount()
            }
            crate::instruments::inflation_swap::PayReceiveInflation::PayFixed => {
                (pv_inflation - pv_fixed)?.amount()
            }
        };

        let duration = t_maturity; // zero-coupon approximation
        let dv01 = -duration * pv_net * 0.0001;

        Ok(dv01)
    }
}
