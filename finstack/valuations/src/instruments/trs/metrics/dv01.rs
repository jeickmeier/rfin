use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::metrics::{GenericParallelDv01, MetricCalculator, MetricContext};
use finstack_core::{Error, Result};

/// Calculates DV01 (interest rate sensitivity) for a TRS.
///
/// DV01 measures the change in present value for a 1 basis point parallel shift in interest rates.
/// This implementation delegates to the generic parallel DV01 calculator based on the TRS variant.
#[derive(Default)]
pub struct TrsDv01Calculator;

impl MetricCalculator for TrsDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        if context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>()
            .is_some()
        {
            return GenericParallelDv01::<EquityTotalReturnSwap>::default().calculate(context);
        }

        if context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
            .is_some()
        {
            return GenericParallelDv01::<FIIndexTotalReturnSwap>::default().calculate(context);
        }

        Err(Error::Input(finstack_core::error::InputError::Invalid))
    }
}
