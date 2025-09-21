//! Quote amount metric for `FxSpot`.
//!
//! Returns the PV value in the quote currency as a scalar.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Returns the quote amount (PV in quote currency).
pub struct QuoteAmountCalculator;

impl MetricCalculator for QuoteAmountCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        Ok(context.base_value.amount())
    }
}
