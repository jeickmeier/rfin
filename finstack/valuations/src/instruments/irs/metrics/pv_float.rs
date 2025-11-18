//! IRS floating leg PV metric.
//!
//! Discounts floating coupons projected from a forward curve, including
//! any quoted spread in basis points.
//! Only includes future cashflows (payment date > as_of date).

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};

/// PV of the floating leg of an IRS.
pub struct FloatLegPvCalculator;

impl MetricCalculator for FloatLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;

        // Use the same discount curve as the main IRS pricer (fixed-leg curve)
        let disc = context.curves.get_discount(&irs.fixed.discount_curve_id)?;

        let pv_money = if irs.is_ois() {
            // OIS / compounded RFR swap: reuse discount-only helper for consistency with npv()
            irs.pv_compounded_float_leg(&disc, as_of)?
        } else {
            // Non-OIS swap: requires forward curve for float leg pricing
            let fwd = context.curves.get_forward(&irs.float.forward_curve_id)?;
            irs.pv_float_leg(&disc, fwd.as_ref(), as_of)?
        };

        Ok(pv_money.amount())
    }
}
