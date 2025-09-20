//! Time to maturity metric for `Repo`.
//!
//! Computes the year fraction from `as_of` to repo maturity using the
//! instrument's day-count convention.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculate time to maturity in years.
pub struct TimeToMaturityCalculator;

impl MetricCalculator for TimeToMaturityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<crate::instruments::repo::Repo>()?;
        let ttm = repo.day_count.year_fraction(
            context.as_of,
            repo.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        Ok(ttm)
    }
}


