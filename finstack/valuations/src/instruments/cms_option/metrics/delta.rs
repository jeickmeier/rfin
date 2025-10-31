//! Delta calculator for CMS options.
//!
//! Computes delta (swap rate sensitivity) using finite differences.
//! Note: Delta for CMS options measures sensitivity to the underlying swap rate.
//! Implementation depends on CMS pricer computing forward swap rates.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Delta calculator for CMS options.
///
/// # Note
///
/// This metric requires the CMS pricer to be fully implemented to compute
/// forward swap rates. Currently returns 0.0 as the pricer is a placeholder.
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, _context: &mut MetricContext) -> Result<f64> {
        // TODO: Implement once CMS pricer computes forward swap rates
        // Delta for CMS options measures sensitivity to the underlying swap rate
        // and requires bumping the forward curve and recomputing the option value
        Ok(0.0)
    }
}

