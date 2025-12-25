//! Delta calculator for equity spot positions.
//!
//! Returns the effective share exposure (shares).

use crate::instruments::equity::Equity;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Delta calculator for equity spot.
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let equity: &Equity = context.instrument_as()?;
        Ok(equity.shares.unwrap_or(1.0))
    }
}
