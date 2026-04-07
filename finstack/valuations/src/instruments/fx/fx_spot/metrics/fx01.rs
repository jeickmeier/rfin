//! FX01 calculator for FX Spot.
//!
//! Computes sensitivity to a 1bp absolute bump in the spot FX rate.

use crate::instruments::fx::fx_spot::FxSpot;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// FX01 calculator for FX Spot.
pub(crate) struct Fx01Calculator;

impl MetricCalculator for Fx01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fx_spot: &FxSpot = context.instrument_as()?;
        let base_notional = fx_spot.effective_notional().amount();
        // 1bp absolute bump in spot rate
        Ok(base_notional * 0.0001)
    }
}
