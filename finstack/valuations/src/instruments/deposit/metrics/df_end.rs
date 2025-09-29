use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};


/// Calculates discount factor at end date for deposits.
///
/// Computes the present value of 1 received at the deposit end date,
/// using the deposit's discount curve and the instrument day count.
pub struct DfEndCalculator;

impl MetricCalculator for DfEndCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let deposit: &Deposit = context.instrument_as()?;

        let disc = context.curves.get_discount_ref(deposit.disc_id.clone())?;
        // Use the curve's own time basis for discounting
        Ok(disc.df_on_date_curve(deposit.end))
    }
}
