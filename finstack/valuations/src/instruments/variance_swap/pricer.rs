use crate::instruments::variance_swap::VarianceSwap;
use crate::instruments::common::GenericDiscountingPricer;

// Use the generic discounting pricer for registry integration
pub type SimpleVarianceSwapDiscountingPricer = GenericDiscountingPricer<VarianceSwap>;
