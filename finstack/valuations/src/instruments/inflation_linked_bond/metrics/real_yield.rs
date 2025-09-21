//! ILB real yield metric calculator.

use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Real yield calculator for ILB
pub struct RealYieldCalculator;

impl MetricCalculator for RealYieldCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let ilb: &InflationLinkedBond = context.instrument_as()?;
        let clean_price = ilb.quoted_clean.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "inflation_linked_bond_quote".to_string(),
            })
        })?;
        ilb.real_yield(clean_price, &context.curves, context.as_of)
    }
}
