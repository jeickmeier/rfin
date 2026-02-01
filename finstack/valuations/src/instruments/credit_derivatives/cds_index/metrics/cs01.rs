//! CDS Index CS01 metric calculator.
//!
//! Approximates the change in PV for a one basis point parallel shift in
//! credit spreads by leveraging the index pricer's CS01 helper.

use crate::instruments::credit_derivatives::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// CS01 calculator for CDS Index
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let idx: &CDSIndex = context.instrument_as()?;
        idx.cs01(&context.curves, context.as_of)
    }
}
