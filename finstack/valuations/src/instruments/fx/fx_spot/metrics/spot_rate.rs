//! Spot rate metric for `FxSpot`.
//!
//! Computes the realized spot rate as `quote_amount / base_amount`, where the
//! `quote_amount` is the instrument PV in quote currency and `base_amount` is
//! the effective base notional.

use crate::instruments::fx::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};

/// Epsilon for floating-point near-zero comparisons.
/// This threshold prevents division by extremely small numbers that would
/// produce numerically unstable results.
const EPSILON: f64 = 1e-15;

/// Calculates the FX spot rate as `quote_amount / base_amount`.
///
/// Returns 0.0 when the base notional is near-zero (within `EPSILON`)
/// to avoid division by very small numbers that would produce
/// numerically unstable results.
pub struct SpotRateCalculator;

impl MetricCalculator for SpotRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fx: &FxSpot = context.instrument_as()?;
        let base_amt = fx.effective_notional().amount();

        // Use epsilon comparison to avoid division by near-zero values
        if base_amt.abs() < EPSILON {
            return Ok(0.0);
        }

        Ok(context.base_value.amount() / base_amt)
    }
}
