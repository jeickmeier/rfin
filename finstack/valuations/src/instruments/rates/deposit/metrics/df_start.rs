use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};

/// Calculates discount factor at effective start date for deposits.
///
/// Computes the present value of 1 received at the deposit effective start date,
/// using the deposit's discount curve.
pub struct DfStartCalculator;

impl MetricCalculator for DfStartCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let deposit: &Deposit = context.instrument_as()?;

        let disc = context.curves.get_discount(&deposit.discount_curve_id)?;
        let effective_start = deposit.effective_start_date()?;
        // Use the curve's own time basis for discounting
        disc.df_on_date_curve(effective_start)
    }
}
