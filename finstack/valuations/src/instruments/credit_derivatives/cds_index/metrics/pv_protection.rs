//! CDS Index protection leg PV metric calculator.
//!
//! Computes present value of the protection leg using the index pricer,
//! which aggregates across pricing modes.

use crate::instruments::credit_derivatives::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Protection leg PV calculator for CDS Index
pub struct ProtectionLegPvCalculator;

impl MetricCalculator for ProtectionLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let idx: &CDSIndex = context.instrument_as()?;
        let pv = idx.pv_protection_leg(&context.curves, context.as_of)?;
        Ok(pv.amount())
    }
}
