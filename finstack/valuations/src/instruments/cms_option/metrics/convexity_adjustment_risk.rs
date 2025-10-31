//! Convexity adjustment risk calculator for CMS options.
//!
//! Computes sensitivity to convexity adjustment parameters using finite differences.
//!
//! # Note
//!
//! CMS options require a convexity adjustment because forward swap rates
//! are not martingales under the forward measure. This metric measures sensitivity
//! to changes in the convexity adjustment methodology or parameters.
//!
//! Implementation depends on the specific convexity adjustment approach used
//! in the CMS pricer (e.g., Hull-White, SABR, etc.).

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Convexity adjustment risk calculator for CMS options.
///
/// # Note
///
/// This metric requires the CMS pricer to be fully implemented with
/// a specific convexity adjustment methodology. Currently returns 0.0
/// as the pricer is a placeholder.
///
/// When implemented, this should bump convexity adjustment parameters
/// (e.g., Hull-White mean reversion, volatility) and measure the resulting
/// change in option value.
pub struct ConvexityAdjustmentRiskCalculator;

impl MetricCalculator for ConvexityAdjustmentRiskCalculator {
    fn calculate(&self, _context: &mut MetricContext) -> Result<f64> {
        // TODO: Implement once CMS pricer has convexity adjustment
        // This would require:
        // 1. Identifying convexity adjustment parameters (e.g., mean reversion, vol)
        // 2. Bumping those parameters
        // 3. Repricing and computing sensitivity
        Ok(0.0)
    }
}

