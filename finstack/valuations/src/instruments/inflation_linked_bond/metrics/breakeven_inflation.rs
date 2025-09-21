//! ILB breakeven inflation metric calculator.

use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Breakeven inflation calculator for ILB
pub struct BreakevenInflationCalculator;

impl MetricCalculator for BreakevenInflationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let _ilb: &InflationLinkedBond = context.instrument_as()?;
        // Requires a nominal bond yield input; not available in `MarketContext`.
        Err(finstack_core::Error::from(
            finstack_core::error::InputError::NotFound { id: "inflation_linked_bond_quote".to_string() },
        ))
    }
}


