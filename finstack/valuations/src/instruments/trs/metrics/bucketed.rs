use crate::instruments::common::metrics::bucketed_dv01::GenericBucketedDv01WithContext;
use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{error::InputError, Error, Result};

/// Bucketed DV01 calculator that dispatches to the appropriate TRS variant.
///
/// The generic helper requires the concrete instrument type at compile time,
/// so we delegate based on the runtime instrument we receive and reuse the
/// existing generic implementations for each TRS variant.
#[derive(Default)]
pub struct TrsBucketedDv01Calculator;

impl MetricCalculator for TrsBucketedDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        if context
            .instrument
            .as_any()
            .downcast_ref::<EquityTotalReturnSwap>()
            .is_some()
        {
            let calc = GenericBucketedDv01WithContext::<EquityTotalReturnSwap>::default();
            return calc.calculate(context);
        }

        if context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
            .is_some()
        {
            let calc = GenericBucketedDv01WithContext::<FIIndexTotalReturnSwap>::default();
            return calc.calculate(context);
        }

        Err(Error::Input(InputError::Invalid))
    }
}
