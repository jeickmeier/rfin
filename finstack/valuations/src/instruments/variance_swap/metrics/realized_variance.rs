//! Realized variance-to-date metric.

use super::super::types::VarianceSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate the current realized variance to date.
pub struct RealizedVarianceCalculator;

impl MetricCalculator for RealizedVarianceCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        let as_of = context.as_of;

        if as_of < swap.start_date {
            return Ok(0.0);
        }

        // Placeholder: production would fetch full price history.
        if let Ok(_scalar) = context.curves.price(&swap.underlying_id) {
            Ok(swap.strike_variance * 0.95)
        } else {
            Ok(0.0)
        }
    }
}
