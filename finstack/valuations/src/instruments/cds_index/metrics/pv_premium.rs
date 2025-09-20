//! CDS Index premium leg PV metric calculator.
//!
//! Computes present value of the premium leg using the index pricer.

use crate::instruments::cds_index::pricing::CDSIndexPricer;
use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Premium leg PV calculator for CDS Index
pub struct PremiumLegPvCalculator;

impl MetricCalculator for PremiumLegPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let idx: &CDSIndex = context.instrument_as()?;
        let pricer = CDSIndexPricer::new();
        let pv = pricer.pv_premium_leg(idx, &context.curves, context.as_of)?;
        Ok(pv.amount())
    }
}
