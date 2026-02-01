//! ILB breakeven inflation metric calculator.

use crate::instruments::fixed_income::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;

/// Breakeven inflation calculator for ILB
pub struct BreakevenInflationCalculator;

impl MetricCalculator for BreakevenInflationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let ilb: &InflationLinkedBond = context.instrument_as()?;
        let curves = context.curves.as_ref();
        let disc_curve = curves.get_discount(ilb.discount_curve_id.as_str())?;

        let day_count = disc_curve.day_count();
        let base_date = disc_curve.base_date();
        let t = day_count.year_fraction(base_date, ilb.maturity, DayCountCtx::default())?;
        let nominal_yield = disc_curve.zero(t);

        ilb.breakeven_inflation(nominal_yield, curves, context.as_of)
    }
}
