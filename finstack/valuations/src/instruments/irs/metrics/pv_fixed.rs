//! IRS fixed leg PV metric.
//!
//! Discounts fixed coupons on the fixed leg using the discount curve.
//! Only includes future cashflows (payment date > as_of date).

use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};

/// PV of the fixed leg of an IRS.
pub struct FixedLegPvCalculator;

impl MetricCalculator for FixedLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;
        let disc = context.curves.get_discount(&irs.fixed.discount_curve_id)?;

        let pv = irs.pv_fixed_leg(&disc, as_of)?;
        Ok(pv.amount())
    }
}
