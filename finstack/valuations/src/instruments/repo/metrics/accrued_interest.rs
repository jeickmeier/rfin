//! Accrued interest metric for repo instruments.
//!
//! Computes accrued interest between the repo start date and the valuation
//! date using the instrument's configured day count and effective rate. The
//! metric returns the currency amount (same units as the cash leg) so it can
//! be combined with other cash-based measures.

use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::prelude::*;

/// Calculator for repo accrued interest (currency amount).
pub struct AccruedInterestCalculator;

impl MetricCalculator for AccruedInterestCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let as_of = context.as_of;
        let (discount_curve_id, day_count, start_date, maturity, notional_amount, effective_rate) = {
            let repo = context.instrument_as::<crate::instruments::repo::Repo>()?;
            (
                repo.disc_id.to_owned(),
                repo.day_count,
                repo.start_date,
                repo.maturity,
                repo.cash_amount.amount(),
                repo.effective_rate(),
            )
        };

        let accrued_amount = if as_of <= start_date {
            0.0
        } else {
            let accrual_end = if as_of >= maturity { maturity } else { as_of };
            let accrual_fraction = day_count.year_fraction(
                start_date,
                accrual_end,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            notional_amount * effective_rate * accrual_fraction
        };

        context.discount_curve_id = Some(discount_curve_id);
        context.day_count = Some(day_count);

        Ok(accrued_amount)
    }
}
