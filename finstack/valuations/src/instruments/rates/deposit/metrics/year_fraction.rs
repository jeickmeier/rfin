use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};

/// Calculates year fraction for deposits.
///
/// Computes the time period between effective start and end dates using the deposit's
/// day count convention. This is fundamental for all other deposit calculations.
/// Errors are propagated (e.g., inverted dates for conventions that enforce it)
/// to avoid silently masking invalid inputs.
///
/// See unit tests and `examples/` for usage.
pub struct YearFractionCalculator;

impl MetricCalculator for YearFractionCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let deposit: &Deposit = context.instrument_as()?;
        let effective_start = deposit.effective_start_date()?;
        let effective_end = deposit.effective_end_date()?;
        deposit.day_count.year_fraction(
            effective_start,
            effective_end,
            finstack_core::dates::DayCountCtx::default(),
        )
    }
}
