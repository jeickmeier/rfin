//! Base amount metric for `FxSpot`.
//!
//! Returns the base notional amount in base currency units.

use crate::instruments::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Returns the base amount (notional) in base currency units.
pub struct BaseAmountCalculator;

impl MetricCalculator for BaseAmountCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx: &FxSpot = context.instrument_as()?;
        Ok(fx.effective_notional().amount())
    }
}
