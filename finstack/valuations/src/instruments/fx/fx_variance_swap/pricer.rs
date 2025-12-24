use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::fx_variance_swap::FxVarianceSwap;

/// Type alias for FX variance swap pricer using generic implementation.
pub type SimpleFxVarianceSwapDiscountingPricer = GenericDiscountingPricer<FxVarianceSwap>;

impl Default for SimpleFxVarianceSwapDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::FxVarianceSwap)
    }
}
