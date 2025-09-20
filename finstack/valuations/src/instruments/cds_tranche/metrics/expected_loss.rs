//! CDS Tranche expected loss metric calculator.
//!
//! Computes the total expected loss at maturity using the Gaussian Copula engine.

use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Expected Loss calculator for CDS Tranche
pub struct ExpectedLossCalculator;

impl MetricCalculator for ExpectedLossCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CdsTranche = context.instrument_as()?;
        if context
            .curves
            .as_ref()
            .credit_index(tranche.credit_index_id)
            .is_ok()
        {
            let pricer = crate::instruments::cds_tranche::pricing::engine::CDSTranchePricer::new();
            pricer.calculate_expected_loss(tranche, context.curves.as_ref())
        } else {
            Ok(0.0)
        }
    }
}
