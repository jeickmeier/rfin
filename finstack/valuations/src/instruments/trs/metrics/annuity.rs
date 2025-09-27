use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Error, Result, F};

/// Calculates the financing annuity for a TRS.
///
/// The financing annuity is the sum of discounted year fractions over all payment periods,
/// multiplied by the notional amount. This represents the present value of a 1 basis point
/// spread over the floating rate.
pub struct FinancingAnnuityCalculator;

impl MetricCalculator for FinancingAnnuityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Some(equity_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>()
        {
            equity_trs.financing_annuity(context.curves.as_ref(), context.as_of)
        } else if let Some(fi_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
        {
            fi_trs.financing_annuity(context.curves.as_ref(), context.as_of)
        } else {
            Err(Error::Input(finstack_core::error::InputError::Invalid))
        }
    }
}
