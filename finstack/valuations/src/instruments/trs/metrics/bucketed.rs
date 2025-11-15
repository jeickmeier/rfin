use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::metrics::{MetricCalculator, MetricContext, UnifiedDv01Calculator};
use finstack_core::{error::InputError, Error, Result};

/// Bucketed DV01 calculator that dispatches to the appropriate TRS variant.
///
/// The generic helper requires the concrete instrument type at compile time,
/// so we delegate based on the runtime instrument we receive and reuse the
/// unified DV01 calculator for each TRS variant.
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
            let calc = UnifiedDv01Calculator::<EquityTotalReturnSwap>::new(
                crate::metrics::Dv01CalculatorConfig::key_rate(),
            );
            return calc.calculate(context);
        }

        if context
            .instrument
            .as_any()
            .downcast_ref::<FIIndexTotalReturnSwap>()
            .is_some()
        {
            let calc = UnifiedDv01Calculator::<FIIndexTotalReturnSwap>::new(
                crate::metrics::Dv01CalculatorConfig::key_rate(),
            );
            return calc.calculate(context);
        }

        Err(Error::Input(InputError::Invalid))
    }
}
