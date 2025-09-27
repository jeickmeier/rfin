// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// Inflation Swap discounting pricer using the generic implementation.
pub type SimpleInflationSwapDiscountingPricer = GenericDiscountingPricer<crate::instruments::InflationSwap>;

impl Default for SimpleInflationSwapDiscountingPricer {
    fn default() -> Self {
        Self::inflation_swap()
    }
}
