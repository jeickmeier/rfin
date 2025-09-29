//! ILB index ratio metric calculator.

use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Index ratio calculator for ILB
pub struct IndexRatioCalculator;

impl MetricCalculator for IndexRatioCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let ilb: &InflationLinkedBond = context.instrument_as()?;
        ilb.index_ratio_from_market(context.as_of, context.curves.as_ref())
    }
}
