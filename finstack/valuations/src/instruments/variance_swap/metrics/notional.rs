//! Variance notional metric.

use super::super::types::VarianceSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate variance notional.
pub struct VarianceNotionalCalculator;

impl MetricCalculator for VarianceNotionalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        Ok(swap.notional.amount())
    }
}
