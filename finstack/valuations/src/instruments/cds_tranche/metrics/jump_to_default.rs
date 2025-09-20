//! CDS Tranche jump-to-default metric calculator.
//!
//! Computes the instantaneous tranche loss if an average constituent defaults.

use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Jump-to-default calculator for CDS Tranche
pub struct JumpToDefaultCalculator;

impl MetricCalculator for JumpToDefaultCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CdsTranche = context.instrument_as()?;
        if context
            .curves
            .as_ref()
            .credit_index(tranche.credit_index_id)
            .is_ok()
        {
            let pricer = crate::instruments::cds_tranche::pricing::engine::CDSTranchePricer::new();
            pricer.calculate_jump_to_default(tranche, context.curves.as_ref(), context.as_of)
        } else {
            Ok(0.0)
        }
    }
}
