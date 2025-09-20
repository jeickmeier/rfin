//! Repo interest amount metric.
//!
//! Computes the accrued interest between start and maturity using the
//! instrument's day count and effective rate.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculate repo interest amount.
pub struct RepoInterestCalculator;

impl MetricCalculator for RepoInterestCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<crate::instruments::repo::Repo>()?;
        let interest = repo.interest_amount()?;
        Ok(interest.amount())
    }
}


