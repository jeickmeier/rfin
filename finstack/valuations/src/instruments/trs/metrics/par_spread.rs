use super::FinancingAnnuityCalculator;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Error, Result};

use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};

/// Calculates the par spread for a TRS (spread that makes NPV = 0).
///
/// The par spread is the spread over the floating rate that makes the net present value
/// of the TRS equal to zero. This is calculated as the ratio of the total return leg PV
/// to the financing annuity, scaled to basis points.
pub struct ParSpreadCalculator;

impl MetricCalculator for ParSpreadCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::FinancingAnnuity]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Financing annuity
        let annuity = FinancingAnnuityCalculator.calculate(context)?;
        if annuity.abs() < 1e-10 {
            return Err(Error::Validation(
                "Financing annuity too small for par spread calculation".into(),
            ));
        }

        // PV of TR leg with zero spread
        let tr_pv = if let Some(equity_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>()
        {
            equity_trs.pv_total_return_leg(context.curves.as_ref(), context.as_of)?
        } else if let Some(fi_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
        {
            fi_trs.pv_total_return_leg(context.curves.as_ref(), context.as_of)?
        } else {
            return Err(Error::Input(finstack_core::error::InputError::Invalid));
        };

        Ok(tr_pv.amount() / annuity * 10000.0)
    }
}
