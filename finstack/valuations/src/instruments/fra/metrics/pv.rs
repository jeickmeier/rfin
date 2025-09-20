//! FRA PV metric calculator.
//!
//! Provides a lightweight PV passthrough that returns the base value already
//! computed by the instrument's pricing implementation. Useful for consistency
//! when requesting metrics-only runs that include PV.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Returns PV from the metric context base value.
pub struct FraPvCalculator;

impl MetricCalculator for FraPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        Ok(context.base_value.amount())
    }
}
