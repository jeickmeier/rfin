//! Effective repo rate metric.
//!
//! Returns the effective rate accounting for any special collateral
//! adjustments configured on the instrument.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;

/// Calculate effective repo rate considering special collateral.
pub struct EffectiveRateCalculator;

impl MetricCalculator for EffectiveRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let repo = context.instrument_as::<crate::instruments::repo::Repo>()?;
        Ok(repo.effective_rate())
    }
}
