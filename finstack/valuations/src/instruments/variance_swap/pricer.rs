use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::variance_swap::VarianceSwap;

// Use the generic discounting pricer for registry integration
pub type SimpleVarianceSwapDiscountingPricer = GenericDiscountingPricer<VarianceSwap>;
