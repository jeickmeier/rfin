//! CDS Tranche upfront metric calculator.
//!
//! Computes the net present value at inception (upfront) using the
//! Gaussian Copula pricing engine if the required credit index data are available.

use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Upfront calculator for CDS Tranche
pub struct UpfrontCalculator;

impl MetricCalculator for UpfrontCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CdsTranche = context.instrument_as()?;
        if context
            .curves
            .as_ref()
            .credit_index(tranche.credit_index_id)
            .is_ok()
        {
            let pricer = crate::instruments::cds_tranche::pricing::engine::CDSTranchePricer::new();
            pricer.calculate_upfront(tranche, context.curves.as_ref(), context.as_of)
        } else {
            Ok(0.0)
        }
    }
}
