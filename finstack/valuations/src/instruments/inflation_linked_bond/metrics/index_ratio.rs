//! ILB index ratio metric calculator.

use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Index ratio calculator for ILB
pub struct IndexRatioCalculator;

impl MetricCalculator for IndexRatioCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let ilb: &InflationLinkedBond = context.instrument_as()?;
        let inflation_index = context
            .curves
            .inflation_index(ilb.inflation_id.as_str())
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "inflation_linked_bond_quote".to_string(),
                })
            })?;

        ilb.index_ratio(context.as_of, &inflation_index)
    }
}
