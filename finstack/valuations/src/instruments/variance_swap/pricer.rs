use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::variance_swap::VarianceSwap;

// Use the generic discounting pricer for registry integration
/// Type alias for variance swap discounting pricer using generic implementation
pub type SimpleVarianceSwapDiscountingPricer = GenericDiscountingPricer<VarianceSwap>;

impl Default for SimpleVarianceSwapDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::VarianceSwap)
    }
}
