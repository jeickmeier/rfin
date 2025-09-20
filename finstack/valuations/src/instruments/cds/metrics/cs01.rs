//! CDS CS01 metric calculator.
//!
//! Approximates the change in PV for a one basis point parallel shift in
//! credit spreads by leveraging the engine's `cs01` helper.

use crate::instruments::cds::CreditDefaultSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// CS01 calculator for CDS
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds: &CreditDefaultSwap = context.instrument_as()?;
        cds.cs01(&context.curves)
    }
}
