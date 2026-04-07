//! Collateral coverage ratio metric for `Repo`.
//!
//! Computes `market_value / required_value` using pre-computed metrics.

use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Calculate collateral coverage ratio (market value / required value).
pub(crate) struct CollateralCoverageCalculator;

impl MetricCalculator for CollateralCoverageCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::CollateralValue, MetricId::RequiredCollateral]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let collateral_value = context
            .computed
            .get(&MetricId::CollateralValue)
            .copied()
            .unwrap_or(0.0);
        let required_value = context
            .computed
            .get(&MetricId::RequiredCollateral)
            .copied()
            .unwrap_or(1.0);

        if required_value == 0.0 {
            return Ok(f64::INFINITY);
        }

        Ok(collateral_value / required_value)
    }
}
