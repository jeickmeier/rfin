use crate::instruments::common::GenericInstrumentPricer;
use crate::instruments::fx_variance_swap::FxVarianceSwap;

/// Type alias for FX variance swap pricer using generic implementation.
pub type SimpleFxVarianceSwapDiscountingPricer = GenericInstrumentPricer<FxVarianceSwap>;

impl Default for SimpleFxVarianceSwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::FxVarianceSwap)
    }
}
