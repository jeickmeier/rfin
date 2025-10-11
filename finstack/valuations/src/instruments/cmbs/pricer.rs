// Using generic pricer implementation to eliminate boilerplate
use crate::instruments::cmbs::Cmbs;
pub use crate::instruments::common::GenericDiscountingPricer;

/// CMBS discounting pricer using the generic implementation.
pub type CmbsDiscountingPricer = GenericDiscountingPricer<Cmbs>;

impl Default for CmbsDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::CMBS)
    }
}
