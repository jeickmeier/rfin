use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Calculates year fraction for deposits.
///
/// Computes the time period between start and end dates using the deposit's
/// day count convention. This is fundamental for all other deposit calculations.
/// Errors are propagated (e.g., inverted dates for conventions that enforce it)
/// to avoid silently masking invalid inputs.
///
/// See unit tests and `examples/` for usage.
pub struct YearFractionCalculator;

impl MetricCalculator for YearFractionCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit: &Deposit = context.instrument_as()?;
        deposit
            .day_count
            .year_fraction(
                deposit.start,
                deposit.end,
                finstack_core::dates::DayCountCtx::default(),
            )
    }
}


