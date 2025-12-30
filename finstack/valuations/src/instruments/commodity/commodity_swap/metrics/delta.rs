//! Delta calculator for commodity swaps.

use crate::instruments::commodity_swap::CommoditySwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCount;
use finstack_core::Result;

/// Delta calculator for commodity swaps (per 1.0 unit of floating price).
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap: &CommoditySwap = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount(swap.discount_curve_id.as_str())?;
        let schedule = swap.payment_schedule(context.as_of)?;
        let sign = if swap.pay_fixed { 1.0 } else { -1.0 };

        let mut delta = 0.0;
        for payment_date in schedule {
            if payment_date < context.as_of {
                continue;
            }
            let t = DayCount::Act365F
                .year_fraction(context.as_of, payment_date, Default::default())?
                .max(0.0);
            let df = disc.df(t);
            delta += sign * swap.notional_quantity * df;
        }

        Ok(delta)
    }
}
