// Using generic pricer implementation to eliminate boilerplate
use crate::instruments::clo::Clo;
pub use crate::instruments::common::GenericDiscountingPricer;

/// CLO discounting pricer using the generic implementation.
pub type CloDiscountingPricer = GenericDiscountingPricer<Clo>;

impl Default for CloDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::CLO)
    }
}
