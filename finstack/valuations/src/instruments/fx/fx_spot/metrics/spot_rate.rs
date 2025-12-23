//! Spot rate metric for `FxSpot`.
//!
//! Computes the realized spot rate as `quote_amount / base_amount`, where the
//! `quote_amount` is the instrument PV in quote currency and `base_amount` is
//! the effective base notional.

use crate::instruments::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};

/// Calculates the FX spot rate as `quote_amount / base_amount`.
pub struct SpotRateCalculator;

impl MetricCalculator for SpotRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fx: &FxSpot = context.instrument_as()?;
        let base_amt = fx.effective_notional().amount();
        if base_amt == 0.0 {
            return Ok(0.0);
        }
        Ok(context.base_value.amount() / base_amt)
    }
}
