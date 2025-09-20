use crate::instruments::deposit::Deposit;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::F;

/// Calculates implied DF(end) from quoted rate.
///
/// Computes DF(end) implied by the quoted simple rate using:
/// DF(end) = DF(start) / (1 + rate × year_fraction).
///
/// # Dependencies
/// Requires `DfStart` and `Yf` metrics to be computed first.
pub struct DfEndFromQuoteCalculator;

impl MetricCalculator for DfEndFromQuoteCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DfStart, MetricId::Yf]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let deposit: &Deposit = context.instrument_as()?;

        let r = deposit.quote_rate.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                id: "deposit_quote_rate".to_string(),
            })
        })?;

        let df_s = context
            .computed
            .get(&MetricId::DfStart)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "deposit_quote_rate".to_string(),
                })
            })?;
        let yf = context
            .computed
            .get(&MetricId::Yf)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "deposit_quote_rate".to_string(),
                })
            })?;

        Ok(df_s / (1.0 + r * yf))
    }
}
