//! Funding risk metric for `Repo`.
//!
//! Approximates the sensitivity of the repo PV to a +1bp change in the
//! instrument's repo rate parameter.

use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculate funding risk (repo rate sensitivity).
pub struct FundingRiskCalculator;

impl MetricCalculator for FundingRiskCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        const ONE_BP: F = 1e-4; // 1 basis point as decimal
        let repo = context.instrument_as::<crate::instruments::repo::Repo>()?;
        let base_pv = repo.value(&context.curves, context.as_of)?.amount();
        let mut repo_bumped = repo.clone();
        repo_bumped.repo_rate += ONE_BP;
        let bumped_pv = repo_bumped.value(&context.curves, context.as_of)?.amount();
        Ok(base_pv - bumped_pv)
    }
}
