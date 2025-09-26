use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::F;

/// Calculates discount factor at start date for deposits.
///
/// Computes the present value of 1 received at the deposit start date,
/// using the deposit's discount curve and the instrument day count.
pub struct DfStartCalculator;

impl MetricCalculator for DfStartCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit: &Deposit = context.instrument_as()?;

        let disc = context
            .curves
            .get_discount_ref(
            deposit.disc_id.clone(),
        )?;
        // Use the curve's own time basis for discounting
        Ok(disc.df_on_date_curve(deposit.start))
    }
}
