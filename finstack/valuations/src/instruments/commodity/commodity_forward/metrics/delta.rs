//! Delta calculator for commodity forwards.

use crate::instruments::commodity_forward::CommodityForward;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCount;
use finstack_core::Result;

/// Delta calculator for commodity forwards (per 1.0 unit of forward price).
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fwd: &CommodityForward = context.instrument_as()?;
        let disc = context
            .curves
            .get_discount(fwd.discount_curve_id.as_str())?;
        let t = DayCount::Act365F
            .year_fraction(context.as_of, fwd.settlement_date, Default::default())?
            .max(0.0);
        let df = disc.df(t);
        Ok(fwd.quantity * fwd.multiplier * df)
    }
}
