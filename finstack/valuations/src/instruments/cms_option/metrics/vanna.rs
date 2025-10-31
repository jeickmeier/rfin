//! Vanna calculator for CMS options.
//!
//! Computes vanna (swap rate vs volatility sensitivity) using finite differences.
//! Note: Requires CMS pricer to be fully implemented.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Vanna calculator for CMS options.
///
/// # Note
///
/// This metric requires the CMS pricer to compute forward swap rates.
/// Currently returns 0.0 as placeholder.
pub struct VannaCalculator;

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, _context: &mut MetricContext) -> Result<f64> {
        // TODO: Implement once CMS pricer computes forward swap rates
        // Vanna for CMS options measures sensitivity to correlation between
        // swap rate movements and volatility changes
        Ok(0.0)
    }
}

