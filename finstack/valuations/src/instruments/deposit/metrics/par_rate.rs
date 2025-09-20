use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::F;

/// Calculates par rate for deposits.
///
/// Computes the simple rate that makes the deposit worth par (face value) at
/// inception using: (DF(start) / DF(end) - 1) / year_fraction.
///
/// # Dependencies
/// Requires `DfStart`, `DfEnd`, and `Yf` metrics to be computed first.
pub struct DepositParRateCalculator;

impl MetricCalculator for DepositParRateCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DfStart, MetricId::DfEnd, MetricId::Yf]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let df_s = context
            .computed
            .get(&MetricId::DfStart)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: "deposit_quote_rate".to_string(),
                })
            })?;
        let df_e = context
            .computed
            .get(&MetricId::DfEnd)
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

        if yf == 0.0 {
            return Ok(0.0);
        }

        Ok((df_s / df_e - 1.0) / yf)
    }
}
