use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Error, Result, F};
use crate::instruments::derivatives::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::instruments::derivatives::trs::pricing::engine::TrsEngine;

/// Calculates IR01 (interest rate sensitivity) for a TRS.
///
/// IR01 measures the change in present value for a 1 basis point parallel shift in interest rates.
/// This implementation approximates the sensitivity using the financing annuity as a proxy.
pub struct TrsIR01Calculator;

impl MetricCalculator for TrsIR01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        // Compute annuity as a proxy for sensitivity
        let bump_size = 0.0001; // 1bp
        let annuity = if let Some(equity_trs) = context.instrument.as_any().downcast_ref::<EquityTotalReturnSwap>() {
            TrsEngine::financing_annuity(&equity_trs.financing, &equity_trs.schedule, equity_trs.notional, context.curves.as_ref(), context.as_of)?
        } else if let Some(fi_trs) = context.instrument.as_any().downcast_ref::<FIIndexTotalReturnSwap>() {
            TrsEngine::financing_annuity(&fi_trs.financing, &fi_trs.schedule, fi_trs.notional, context.curves.as_ref(), context.as_of)?
        } else {
            return Err(Error::Input(finstack_core::error::InputError::Invalid));
        };
        Ok(annuity * bump_size)
    }
}


