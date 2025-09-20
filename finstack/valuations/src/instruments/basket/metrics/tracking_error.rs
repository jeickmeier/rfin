//! Tracking error metric calculator.
//!
//! Placeholder implementation: returns 0.0 until historical series support is
//! integrated. Avoid magic constants.

use crate::instruments::basket::types::Basket;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate tracking error vs benchmark (requires benchmark data)
pub struct TrackingErrorCalculator;

impl MetricCalculator for TrackingErrorCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> { 
        let _basket = context.instrument_as::<Basket>()?; 
        Ok(0.0) 
    }
}


