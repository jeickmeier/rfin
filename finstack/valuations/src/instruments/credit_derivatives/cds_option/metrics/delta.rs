//! Delta metric for `CdsOption`.

use crate::instruments::credit_derivatives::cds_option::CdsOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Delta calculator for credit options on CDS spreads.
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CdsOption = context.instrument_as()?;
        option.delta(&context.curves, context.as_of)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
