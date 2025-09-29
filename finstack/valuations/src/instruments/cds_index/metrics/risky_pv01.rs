//! CDS Index risky PV01 metric calculator.
//!
//! Computes the change in present value for a one basis point change in the
//! premium spread. Delegates to `CDSIndexPricer` which aggregates by mode.

use crate::instruments::cds_index::CDSIndex;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result};

/// Risky PV01 calculator for CDS Index
pub struct RiskyPv01Calculator;

impl MetricCalculator for RiskyPv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let idx: &CDSIndex = context.instrument_as()?;
        idx.risky_pv01(&context.curves, context.as_of)
    }
}
