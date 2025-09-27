// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// Basis Swap discounting pricer using the generic implementation.
pub type SimpleBasisSwapDiscountingPricer = GenericDiscountingPricer<crate::instruments::basis_swap::BasisSwap>;

impl Default for SimpleBasisSwapDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}


