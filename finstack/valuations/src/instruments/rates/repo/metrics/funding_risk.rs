//! Funding risk metric for `Repo`.
//!
//! Approximates the sensitivity of the repo PV to a +1bp change in the
//! instrument's repo rate parameter.

use crate::instruments::common_impl::traits::Instrument;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;
use rust_decimal::Decimal;

/// Calculate funding risk (repo rate sensitivity).
pub(crate) struct FundingRiskCalculator;

impl MetricCalculator for FundingRiskCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        const ONE_BP: f64 = 1e-4; // 1 basis point as decimal
        let repo = context.instrument_as::<crate::instruments::rates::repo::Repo>()?;
        let base_pv = repo.value(&context.curves, context.as_of)?.amount();
        let mut repo_bumped = repo.clone();
        repo_bumped.repo_rate +=
            Decimal::try_from(ONE_BP).map_err(|_| finstack_core::InputError::ConversionOverflow)?;
        let bumped_pv = repo_bumped.value(&context.curves, context.as_of)?.amount();
        Ok(base_pv - bumped_pv)
    }
}
