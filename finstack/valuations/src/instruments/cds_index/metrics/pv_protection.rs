//! CDS Index protection leg PV metric calculator.
//!
//! Computes present value of the protection leg using the index pricer,
//! which aggregates across pricing modes.

use crate::instruments::cds_index::CDSIndex;
use crate::instruments::cds_index::pricing::CDSIndexPricer;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Protection leg PV calculator for CDS Index
pub struct ProtectionLegPvCalculator;

impl MetricCalculator for ProtectionLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let idx: &CDSIndex = context.instrument_as()?;
        let pricer = CDSIndexPricer::new();
        let pv = pricer.pv_protection_leg(idx, &context.curves, context.as_of)?;
        Ok(pv.amount())
    }
}


