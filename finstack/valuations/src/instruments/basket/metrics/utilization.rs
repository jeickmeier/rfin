//! Utilization metric calculator.
//!
//! Computes current utilization vs creation unit size when shares outstanding
//! are provided; otherwise returns 0.

use crate::instruments::basket::types::Basket;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate current utilization vs creation unit size
pub struct UtilizationCalculator;

impl MetricCalculator for UtilizationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        if let Some(shares) = basket.shares_outstanding {
            Ok(shares / basket.creation_unit_size)
        } else {
            Ok(0.0)
        }
    }
}
