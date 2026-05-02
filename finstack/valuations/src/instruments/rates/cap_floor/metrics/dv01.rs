//! Market DV01 calculator for caps/floors.

use crate::instruments::rates::cap_floor::CapFloor;
use crate::metrics::{
    Dv01CalculatorConfig, MetricCalculator, MetricContext, UnifiedDv01Calculator,
};
use finstack_core::Result;

/// Cap/floor model-basis DV01.
///
/// This reports the raw finite-difference sensitivity to the model discount
/// and projection curves. Quote-basis RFR cap/floor risk requires rebumping the
/// underlying OIS quote stack and rebootstrap, which is not yet represented by
/// the generic curve-bump API; avoid applying fixture-tuned quote scalars here.
pub(crate) struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let inner =
            UnifiedDv01Calculator::<CapFloor>::new(Dv01CalculatorConfig::parallel_combined());
        inner.calculate(context)
    }
}
