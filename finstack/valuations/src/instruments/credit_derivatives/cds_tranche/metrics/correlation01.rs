//! CDS Tranche correlation sensitivity metric calculator.
//!
//! Measures PV sensitivity to a shift in the base correlation curve (per 1% correlation change).

use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Correlation01 calculator for CDS Tranche
pub struct Correlation01Calculator;

impl MetricCalculator for Correlation01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CdsTranche = context.instrument_as()?;
        tranche.correlation_delta(&context.curves, context.as_of)
    }
}
