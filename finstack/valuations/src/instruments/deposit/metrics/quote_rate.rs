use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext};

/// Quoted rate passthrough for deposits.
///
/// Returns the quoted simple rate from the instrument.
///
/// # Errors
/// Returns an error if the deposit does not have a quoted rate set.
pub struct QuoteRateCalculator;

impl MetricCalculator for QuoteRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let deposit: &Deposit = context.instrument_as()?;
        deposit.quote_rate.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "QuoteRate (deposit has no quoted rate set)".to_string(),
            })
        })
    }
}
