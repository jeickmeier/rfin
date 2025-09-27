//! CDS Tranche Spread DV01 metric calculator.
//!
//! Measures the premium-leg PV change for a 1bp change in the running coupon.

use crate::instruments::cds_tranche::CdsTranche;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Spread DV01 calculator for CDS Tranche
pub struct SpreadDv01Calculator;

impl MetricCalculator for SpreadDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let tranche: &CdsTranche = context.instrument_as()?;
        if context
            .curves
            .as_ref()
            .credit_index(&tranche.credit_index_id)
            .is_ok()
        {
            tranche.spread_dv01(&context.curves, context.as_of)
        } else {
            Ok(0.0)
        }
    }
}
