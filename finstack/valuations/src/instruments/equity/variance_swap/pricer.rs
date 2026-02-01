use crate::instruments::common::GenericInstrumentPricer;
use crate::instruments::equity::variance_swap::VarianceSwap;

// Use the generic discounting pricer for registry integration
/// Type alias for variance swap discounting pricer using generic implementation
pub type SimpleVarianceSwapDiscountingPricer = GenericInstrumentPricer<VarianceSwap>;

impl Default for SimpleVarianceSwapDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::VarianceSwap)
    }
}
