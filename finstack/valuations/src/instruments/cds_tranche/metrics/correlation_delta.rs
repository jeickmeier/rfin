//! CDS Tranche correlation delta metric calculator.
//!
//! Measures PV sensitivity to a shift in the base correlation curve.

use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Correlation delta calculator for CDS Tranche
pub struct CorrelationDeltaCalculator;

impl MetricCalculator for CorrelationDeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CdsTranche = context.instrument_as()?;
        let pricer = crate::instruments::cds_tranche::pricing::engine::CDSTranchePricer::new();
        pricer.calculate_correlation_delta(tranche, context.curves.as_ref(), context.as_of)
    }
}
