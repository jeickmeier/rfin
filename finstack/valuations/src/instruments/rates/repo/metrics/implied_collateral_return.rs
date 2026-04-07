//! Implied collateral return metric for `Repo`.
//!
//! Computes an implied annualized return based on the ratio of current
//! collateral value to required collateral, normalized by time to maturity.
//!
//! # Market Standard
//!
//! Uses business-day adjusted maturity date for consistency with PV and
//! other metric calculations.

use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Calculate implied collateral return (mark-to-market gain/loss on collateral).
///
/// Uses business-day adjusted maturity for consistency with PV calculations.
pub(crate) struct ImpliedCollateralReturnCalculator;

impl MetricCalculator for ImpliedCollateralReturnCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::CollateralValue, MetricId::RequiredCollateral]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let repo = context.instrument_as::<crate::instruments::rates::repo::Repo>()?;
        let collateral_value = context
            .computed
            .get(&MetricId::CollateralValue)
            .copied()
            .unwrap_or(0.0);
        let required_value = context
            .computed
            .get(&MetricId::RequiredCollateral)
            .copied()
            .unwrap_or(0.0);

        // Use adjusted maturity for consistency with PV and interest calculations
        let (_, adj_maturity) = repo.adjusted_dates()?;

        let ttm = repo.day_count.year_fraction(
            context.as_of,
            adj_maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if ttm <= 0.0 || required_value == 0.0 {
            return Ok(0.0);
        }

        let return_rate = (collateral_value / required_value - 1.0) / ttm;
        Ok(return_rate)
    }
}
