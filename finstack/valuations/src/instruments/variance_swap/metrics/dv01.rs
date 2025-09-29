//! DV01 metric (PV sensitivity to a 1bp parallel rate move).

use super::super::types::VarianceSwap;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result};

/// Calculate DV01 (sensitivity to 1bp move in interest rates).
pub struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        let as_of = context.as_of;

        if as_of >= swap.maturity {
            return Ok(0.0);
        }

        let pv = swap.value(&context.curves, as_of)?;
        let ttm = swap
            .day_count
            .year_fraction(as_of, swap.maturity, Default::default())?;
        // Signed DV01: dPV/d(rate) ≈ -PV * T
        Ok(-pv.amount() * ttm * 0.0001)
    }
}
