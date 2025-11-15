use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
use crate::metrics::{MetricCalculator, MetricContext, UnifiedDv01Calculator};
use finstack_core::Result;

/// Calculates DV01 (interest rate sensitivity) for Total Return Swaps.
///
/// DV01 measures the change in present value for a 1 basis point parallel shift in the
/// financing discount curve. This calculator dispatches to `UnifiedDv01Calculator` based
/// on the TRS variant (Equity or Fixed Income Index).
///
/// Both EquityTotalReturnSwap and FIIndexTotalReturnSwap share the "TRS" instrument type,
/// so this wrapper handles runtime dispatching to the appropriate generic implementation.
#[derive(Default)]
pub struct TrsDv01Calculator;

impl MetricCalculator for TrsDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Try Equity TRS first
        if let Ok(result) = UnifiedDv01Calculator::<EquityTotalReturnSwap>::new(
            crate::metrics::Dv01CalculatorConfig::parallel_combined(),
        )
        .calculate(context)
        {
            return Ok(result);
        }

        // Fall back to FI Index TRS
        UnifiedDv01Calculator::<FIIndexTotalReturnSwap>::new(
            crate::metrics::Dv01CalculatorConfig::parallel_combined(),
        )
        .calculate(context)
    }
}
