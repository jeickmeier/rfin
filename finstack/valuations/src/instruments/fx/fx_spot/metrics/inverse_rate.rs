//! Inverse spot rate metric for `FxSpot`.
//!
//! Computes the inverse of the realized spot rate when the spot rate is
//! non-zero. Returns 0.0 in degenerate cases where division would be unstable.

use crate::instruments::fx::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};

/// Epsilon for floating-point near-zero comparisons.
/// This threshold prevents division by extremely small numbers that would
/// produce numerically unstable results.
const EPSILON: f64 = 1e-15;

/// Calculates the inverse of the spot rate (base per quote) if non-zero.
///
/// Returns 0.0 when:
/// - Base notional is near-zero (within `EPSILON`)
/// - Computed spot rate is near-zero (within `EPSILON`)
///
/// This prevents division by very small numbers that would produce
/// numerically unstable or misleading results.
pub struct InverseRateCalculator;

impl MetricCalculator for InverseRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let fx: &FxSpot = context.instrument_as()?;
        let base_amt = fx.effective_notional().amount();

        // Use epsilon comparison to avoid division by near-zero values
        if base_amt.abs() < EPSILON {
            return Ok(0.0);
        }

        let spot = context.base_value.amount() / base_amt;

        // Use epsilon comparison for spot rate as well
        if spot.abs() < EPSILON {
            Ok(0.0)
        } else {
            Ok(1.0 / spot)
        }
    }
}
