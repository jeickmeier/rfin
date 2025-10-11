// Using generic pricer implementation to eliminate boilerplate
use crate::instruments::abs::Abs;
pub use crate::instruments::common::GenericDiscountingPricer;

/// ABS discounting pricer using the generic implementation.
pub type AbsDiscountingPricer = GenericDiscountingPricer<Abs>;

impl Default for AbsDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::ABS)
    }
}
