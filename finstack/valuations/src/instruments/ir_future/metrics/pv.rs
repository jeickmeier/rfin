//! IR Future PV metric calculator.
//!
//! Provides a lightweight PV passthrough that returns the base value already
//! computed by the instrument's pricing implementation.

use crate::metrics::{MetricCalculator, MetricContext};


/// Returns PV from the metric context base value.
pub struct IrFuturePvCalculator;

impl MetricCalculator for IrFuturePvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        Ok(context.base_value.amount())
    }
}
