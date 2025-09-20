//! Inverse spot rate metric for `FxSpot`.
//!
//! Computes the inverse of the realized spot rate when the spot rate is
//! non-zero. Returns 0.0 in degenerate cases.

use crate::instruments::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Calculates the inverse of the spot rate (base per quote) if non-zero.
pub struct InverseRateCalculator;

impl MetricCalculator for InverseRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let fx: &FxSpot = context.instrument_as()?;
        let base_amt = fx.effective_notional().amount();
        if base_amt == 0.0 {
            return Ok(0.0);
        }
        let spot = context.base_value.amount() / base_amt;
        if spot == 0.0 {
            Ok(0.0)
        } else {
            Ok(1.0 / spot)
        }
    }
}


