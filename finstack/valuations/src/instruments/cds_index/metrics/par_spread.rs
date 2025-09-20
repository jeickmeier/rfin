//! CDS Index par spread metric calculator.
//!
//! Computes the fixed spread in basis points that sets the index NPV to zero.
//! Delegates to the `CDSIndexPricer` which handles both pricing modes.

use crate::instruments::cds_index::CDSIndex;
use crate::instruments::cds_index::pricing::CDSIndexPricer;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Par spread calculator for CDS Index
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let idx: &CDSIndex = context.instrument_as()?;
        let pricer = CDSIndexPricer::new();
        pricer.par_spread(idx, &context.curves, context.as_of)
    }
}


