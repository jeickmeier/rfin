//! Delta calculator for commodity forwards.
//!
//! Delta measures the sensitivity of the forward's NPV to changes in the forward price.
//! For a commodity forward with NPV = sign(position) × (F - K) × Q × M × DF:
//!
//! Delta = ∂NPV/∂F = sign(position) × Q × M × DF
//!
//! Where:
//! - sign(position) = +1 for long, -1 for short
//! - Q = quantity
//! - M = multiplier
//! - DF = discount factor to settlement

use crate::instruments::commodity_forward::CommodityForward;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCount;
use finstack_core::Result;

/// Delta calculator for commodity forwards (per 1.0 unit of forward price).
///
/// Returns the change in NPV for a 1.0 unit increase in the forward price,
/// accounting for position direction (long = positive delta, short = negative delta).
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fwd: &CommodityForward = context.instrument_as()?;

        // Expired forward has zero delta
        if fwd.settlement_date < context.as_of {
            return Ok(0.0);
        }

        let disc = context
            .curves
            .get_discount(fwd.discount_curve_id.as_str())?;
        let t = DayCount::Act365F
            .year_fraction(context.as_of, fwd.settlement_date, Default::default())?
            .max(0.0);
        let df = disc.df(t);

        // Delta = sign(position) × Q × M × DF
        Ok(fwd.position.sign() * fwd.quantity * fwd.multiplier * df)
    }
}
