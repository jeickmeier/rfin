//! CDS Tranche CS01 metric calculator.
//!
//! Approximates the change in PV for a one basis point parallel shift in
//! credit spreads by leveraging the tranche engine's CS01 helper.

use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// CS01 calculator for CDS Tranche
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CdsTranche = context.instrument_as()?;
        let pricer = crate::instruments::cds_tranche::pricing::engine::CDSTranchePricer::new();
        pricer.calculate_cs01(tranche, context.curves.as_ref(), context.as_of)
    }
}
