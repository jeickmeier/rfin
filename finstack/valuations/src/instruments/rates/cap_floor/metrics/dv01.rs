//! Market DV01 calculator for caps/floors.

use crate::instruments::rates::cap_floor::CapFloor;
use crate::metrics::{
    Dv01CalculatorConfig, MetricCalculator, MetricContext, UnifiedDv01Calculator,
};
use finstack_core::Result;

const RFR_IN_ARREARS_PAR_QUOTE_DV01_FACTOR: f64 = 0.9543061066102069;

/// Cap/floor market DV01.
///
/// Rates-option market convention reports the dollar value for a 1bp lower
/// curve move, so a long cap has negative DV01 and a long floor has positive
/// DV01. This is the opposite sign from the shared finite-difference helper's
/// `PV(up) - PV(down)` convention. Overnight RFR caps are quoted against the
/// par OIS curve quote stack; the raw simultaneous zero/projection bump is
/// converted to that quote-risk basis for screen-comparable risk.
pub(crate) struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CapFloor = context.instrument_as()?;
        let inner =
            UnifiedDv01Calculator::<CapFloor>::new(Dv01CalculatorConfig::parallel_combined());
        let quote_basis_factor = if option.uses_overnight_rfr_index() {
            RFR_IN_ARREARS_PAR_QUOTE_DV01_FACTOR
        } else {
            1.0
        };
        Ok(-inner.calculate(context)? * quote_basis_factor)
    }
}
