use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Error, Result};

/// Calculates IR01 (interest rate sensitivity) for a TRS.
///
/// IR01 measures the change in present value for a 1 basis point parallel shift in interest rates.
/// This implementation approximates the sensitivity using the financing annuity as a proxy.
pub struct TrsIR01Calculator;

impl MetricCalculator for TrsIR01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Compute annuity as a proxy for sensitivity
        let bump_size = 0.0001; // 1bp
        let annuity = if let Some(equity_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>()
        {
            equity_trs.financing_annuity(context.curves.as_ref(), context.as_of)?
        } else if let Some(fi_trs) = context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
        {
            fi_trs.financing_annuity(context.curves.as_ref(), context.as_of)?
        } else {
            return Err(Error::Input(finstack_core::error::InputError::Invalid));
        };
        Ok(annuity * bump_size)
    }
}
